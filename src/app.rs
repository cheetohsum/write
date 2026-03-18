use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use tui_textarea::CursorMove;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use crate::config::LlmConfig;
use crate::editor::EditorState;
use crate::keybindings::{self, Action};
use crate::llm::{self, LlmRequest, LlmResponse};
use crate::theme;
use crate::ui;

const DEBOUNCE_IDLE_MS: u64 = 1500;
const DEBOUNCE_WORD_MS: u64 = 600;
const DEBOUNCE_SENTENCE_MS: u64 = 300;
const DEBOUNCE_NEWLINE_MS: u64 = 300;
const DEBOUNCE_MICRO_MS: u64 = 500;
const DEBOUNCE_RATE_LIMITED_MS: u64 = 8000;
const STARTUP_ANIM_MS: u64 = 900;
const TRANSITION_MS: u64 = 350;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Startup,
    Settings,
    Editor,
    QuitConfirm,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum LlmStatus {
    Disabled,
    Idle,
    Waiting,
    Cleaning,
    Applied,
    Error,
    Off,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RevealStyle {
    TopDown,
    Scatter,
    ZoomIn,
    ZoomOut,
}

pub struct StartupAnim {
    pub start: Instant,
}

/// Visual dissolve on changed text positions (render-buffer only, no cursor movement)
pub struct TextDissolve {
    pub changed_positions: Vec<(usize, usize)>, // (doc_row, doc_col)
    pub resolve_times: Vec<u64>,                // ms at which each position resolves
    pub start: Instant,
}

pub struct TransitionAnim {
    pub start: Instant,
    pub style: RevealStyle,
}

/// Saved state for a page we navigated away from.
pub struct PageEntry {
    pub file_path: PathBuf,
    pub display_name: String,
    pub content: String,
    pub cursor: (usize, usize),
    pub modified: bool,
    pub last_sent_hash: String,
}

pub struct AppState<'a> {
    pub screen: Screen,
    pub llm_config: Option<LlmConfig>,
    pub editor: EditorState<'a>,
    pub doc_title: String,
    pub output_dir: String,
    pub dir_input: TextArea<'a>,
    pub title_input: TextArea<'a>,
    pub startup_field: u8,
    pub llm_status: LlmStatus,
    pub llm_status_flash: Option<Instant>,
    pub llm_enabled: bool,
    pub just_saved: bool,
    pub save_flash_until: Option<Instant>,
    pub last_keypress: Instant,
    pub debounce_duration: Duration,
    pub in_flight: bool,
    pub should_quit: bool,
    pub words_since_send: u8,
    pub last_click: Option<(Instant, u16, u16)>,
    pub text_dissolve: Option<TextDissolve>,
    pub startup_anim: Option<StartupAnim>,
    pub transition: Option<TransitionAnim>,
    pub editor_area: Rect,
    // Graph-node navigation
    pub current_file: PathBuf,
    pub current_page_name: String,
    pub page_stack: Vec<PageEntry>,
    // Settings screen
    pub settings_field: u8,
    pub settings_inputs: [TextArea<'a>; 3],
    pub settings_link_rects: [Rect; 3],
}

impl<'a> AppState<'a> {
    pub fn new(llm_config: Option<LlmConfig>) -> Self {
        let default_dir = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        let mut dir_input = TextArea::default();
        dir_input.insert_str(&default_dir);
        dir_input.set_style(theme::input_active());
        dir_input.set_cursor_style(theme::cursor());

        let mut title_input = TextArea::default();
        title_input.set_style(theme::input_inactive());

        let llm_enabled = llm_config.is_some();
        let llm_status = if llm_config.is_some() {
            LlmStatus::Idle
        } else {
            LlmStatus::Disabled
        };

        AppState {
            screen: Screen::Startup,
            llm_config,
            editor: EditorState::new(),
            doc_title: String::new(),
            output_dir: default_dir,
            dir_input,
            title_input,
            startup_field: 1,
            llm_status,
            llm_status_flash: None,
            llm_enabled,
            just_saved: false,
            save_flash_until: None,
            last_keypress: Instant::now(),
            debounce_duration: Duration::from_millis(DEBOUNCE_IDLE_MS),
            in_flight: false,
            should_quit: false,
            words_since_send: 0,
            last_click: None,
            text_dissolve: None,
            editor_area: Rect::default(),
            startup_anim: Some(StartupAnim {
                start: Instant::now(),
            }),
            transition: None,
            current_file: PathBuf::new(),
            current_page_name: String::new(),
            page_stack: Vec::new(),
            settings_field: 0,
            settings_inputs: [
                TextArea::default(),
                TextArea::default(),
                TextArea::default(),
            ],
            settings_link_rects: [Rect::default(); 3],
        }
    }

    pub fn breadcrumb(&self) -> String {
        if self.page_stack.is_empty() {
            self.doc_title.clone()
        } else {
            let mut parts: Vec<String> = self
                .page_stack
                .iter()
                .map(|e| e.display_name.clone())
                .collect();
            parts.push(self.current_page_name.clone());
            parts.join(" › ")
        }
    }

    pub fn is_nested(&self) -> bool {
        !self.page_stack.is_empty()
    }
}

pub async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    llm_config: Option<LlmConfig>,
) -> Result<()> {
    let mut state = AppState::new(llm_config.clone());

    // LLM channels are created on demand when entering the editor
    let (mut llm_tx, mut llm_rx): (
        Option<mpsc::Sender<LlmRequest>>,
        Option<mpsc::Receiver<LlmResponse>>,
    ) = (None, None);

    loop {
        let term_width = terminal.size()?.width;
        state.editor.wrap_width = term_width.saturating_sub(10);

        // Clear startup animation when done
        if let Some(ref anim) = state.startup_anim {
            if anim.start.elapsed().as_millis() as u64 >= STARTUP_ANIM_MS {
                state.startup_anim = None;
            }
        }

        // Complete transition when done
        if let Some(ref trans) = state.transition {
            if trans.start.elapsed().as_millis() as u64 >= TRANSITION_MS {
                state.transition = None;
            }
        }

        // Render
        terminal.draw(|f| ui::render(f, &mut state))?;

        // Handle save flash timeout
        if let Some(until) = state.save_flash_until {
            if Instant::now() > until {
                state.just_saved = false;
                state.save_flash_until = None;
            }
        }

        // Check for LLM responses
        if let Some(ref mut rx) = llm_rx {
            if let Ok(response) = rx.try_recv() {
                state.in_flight = false;
                handle_llm_response(&mut state, response);
            }
        }

        // Check debounce
        if state.screen == Screen::Editor
            && state.llm_enabled
            && !state.in_flight
            && state.editor.modified
            && state.transition.is_none()
        {
            let elapsed = state.last_keypress.elapsed();
            if elapsed >= state.debounce_duration {
                let current_hash = state.editor.content_hash();
                if current_hash != state.editor.last_sent_hash
                    && !state.editor.content().trim().is_empty()
                {
                    if let Some(ref tx) = llm_tx {
                        let text = state.editor.content();
                        state.editor.last_sent_hash = current_hash.clone();
                        state.in_flight = true;
                        state.words_since_send = 0;
                        state.llm_status = LlmStatus::Cleaning;
                        state.llm_status_flash = Some(Instant::now());
                        let _ = tx.try_send(LlmRequest {
                            text,
                            hash: current_hash,
                        });
                    }
                }
            }
        }

        let status_flashing = state.llm_status_flash.map_or(false, |t| t.elapsed().as_millis() < 1500);
        let text_dissolving = state.text_dissolve.as_ref().map_or(false, |d| d.start.elapsed().as_millis() < 450);
        if !text_dissolving && state.text_dissolve.is_some() {
            state.text_dissolve = None;
        }
        let animating = state.startup_anim.is_some()
            || state.transition.is_some()
            || status_flashing
            || text_dissolving;
        let poll_duration = if animating {
            Duration::from_millis(30)
        } else {
            Duration::from_millis(100)
        };

        if event::poll(poll_duration)? {
            match event::read()? {
                Event::Key(key) => {
                    if state.startup_anim.is_some() {
                        if matches!(key.code, KeyCode::Esc)
                            || matches!(
                                (key.code, key.modifiers),
                                (KeyCode::Char('q'), KeyModifiers::CONTROL)
                            )
                        {
                            state.should_quit = true;
                        }
                        if state.should_quit {
                            break;
                        }
                        continue;
                    }

                    if state.transition.is_some() {
                        continue;
                    }

                    state.last_keypress = Instant::now();

                    let prev_screen = state.screen.clone();
                    handle_key(&mut state, key)?;

                    // Handle screen transitions
                    if prev_screen != state.screen {
                        match (&prev_screen, &state.screen) {
                            (_, Screen::Editor) => {
                                // Entering editor — spawn LLM if configured
                                if let Some(ref cfg) = state.llm_config {
                                    let (tx, rx) = llm::spawn_llm_task(cfg.clone());
                                    llm_tx = Some(tx);
                                    llm_rx = Some(rx);
                                }
                            }
                            (Screen::Editor, _) => {
                                // Leaving editor — drop LLM channels
                                llm_tx = None;
                                llm_rx = None;
                            }
                            _ => {}
                        }
                    }

                    if state.should_quit {
                        break;
                    }
                }
                Event::Mouse(mouse) => {
                    // Settings screen: click links to open provider URLs
                    if state.screen == Screen::Settings
                        && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                    {
                        for (i, rect) in state.settings_link_rects.iter().enumerate() {
                            if rect.width > 0
                                && mouse.column >= rect.left()
                                && mouse.column < rect.right()
                                && mouse.row >= rect.top()
                                && mouse.row < rect.bottom()
                            {
                                crate::config::open_provider_url(i);
                            }
                        }
                    }
                    if state.screen == Screen::Editor
                        && state.transition.is_none()
                        && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                    {
                        let area = state.editor_area;
                        if mouse.column >= area.left()
                            && mouse.column < area.right()
                            && mouse.row >= area.top()
                            && mouse.row < area.bottom()
                        {
                            // Check for double-click (within 400ms, same position)
                            let is_double = state.last_click.map_or(false, |(t, lx, ly)| {
                                t.elapsed().as_millis() < 400
                                    && lx == mouse.column
                                    && ly == mouse.row
                            });

                            // Position cursor at click
                            let click_row = (mouse.row - area.top()) as usize;
                            let click_col = (mouse.column - area.left()) as usize;
                            let (cur_row, _) = state.editor.textarea.cursor();
                            let viewport_h = area.height as usize;
                            let scroll_top =
                                cur_row.saturating_sub(viewport_h.saturating_sub(1));
                            let target_row = (scroll_top + click_row)
                                .min(state.editor.textarea.lines().len().saturating_sub(1));

                            state.editor.textarea.move_cursor(CursorMove::Top);
                            state.editor.textarea.move_cursor(CursorMove::Head);
                            for _ in 0..target_row {
                                state.editor.textarea.move_cursor(CursorMove::Down);
                            }
                            let line_len = state
                                .editor
                                .textarea
                                .lines()
                                .get(target_row)
                                .map(|l| l.len())
                                .unwrap_or(0);
                            let target_col = click_col.min(line_len);
                            for _ in 0..target_col {
                                state.editor.textarea.move_cursor(CursorMove::Forward);
                            }

                            // Double-click on a [[link]] → navigate into it
                            if is_double {
                                if state.editor.find_link_at_cursor().is_some() {
                                    let _ = navigate_into_link(&mut state);
                                }
                                state.last_click = None;
                            } else {
                                state.last_click =
                                    Some((Instant::now(), mouse.column, mouse.row));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_key(state: &mut AppState, key: KeyEvent) -> Result<()> {
    match state.screen {
        Screen::Startup => handle_startup_key(state, key),
        Screen::Settings => handle_settings_key(state, key),
        Screen::Editor => handle_editor_key(state, key),
        Screen::QuitConfirm => handle_modal_key(state, key),
    }
}

fn handle_startup_key(state: &mut AppState, key: KeyEvent) -> Result<()> {
    let action = keybindings::map_startup_key(key);
    match action {
        Action::Tab => {
            state.startup_field = (state.startup_field + 1) % 3;
        }
        Action::Confirm => {
            if state.startup_field == 2 {
                // Open settings
                init_settings_inputs(state);
                state.screen = Screen::Settings;
                state.transition = Some(TransitionAnim {
                    start: Instant::now(),
                    style: RevealStyle::Scatter,
                });
                return Ok(());
            }

            let dir = state.dir_input.lines().join("");
            let title = state.title_input.lines().join("");

            if title.trim().is_empty() {
                return Ok(());
            }

            state.output_dir = dir;
            state.doc_title = sanitize_filename(&title);
            state.current_file =
                PathBuf::from(&state.output_dir).join(format!("{}.md", state.doc_title));
            state.current_page_name = state.doc_title.clone();

            // Load existing content if file exists
            if state.current_file.exists() {
                if let Ok(content) = fs::read_to_string(&state.current_file) {
                    state.editor.set_content_with_cursor(&content, 0, 0);
                    state.editor.modified = false;
                }
            }

            state.screen = Screen::Editor;
            state.transition = Some(TransitionAnim {
                start: Instant::now(),
                style: RevealStyle::Scatter,
            });
        }
        Action::Quit => {
            state.should_quit = true;
        }
        Action::ForwardToEditor(key_event) => {
            if state.startup_field == 0 {
                state.dir_input.input(key_event);
            } else if state.startup_field == 1 {
                state.title_input.input(key_event);
            }
            // field 2 (settings button): ignore text input
        }
        _ => {}
    }
    Ok(())
}

fn handle_editor_key(state: &mut AppState, key: KeyEvent) -> Result<()> {
    let action = keybindings::map_editor_key(key);
    match action {
        Action::Save => {
            save_document(state)?;
        }
        Action::Back => {
            if state.is_nested() {
                navigate_back(state)?;
            } else {
                return_to_startup(state)?;
            }
        }
        Action::Quit => {
            if state.editor.modified {
                state.screen = Screen::QuitConfirm;
            } else {
                state.should_quit = true;
            }
        }
        Action::ToggleLlm => {
            if state.llm_config.is_some() {
                state.llm_enabled = !state.llm_enabled;
                state.llm_status = if state.llm_enabled {
                    LlmStatus::Idle
                } else {
                    LlmStatus::Off
                };
            }
        }
        Action::CreateLink => {
            state.editor.create_link_at_cursor();
        }
        Action::OpenLink => {
            navigate_into_link(state)?;
        }
        Action::ForwardToEditor(key_event) => {
            // Ctrl+A: select all (tui-textarea maps it to head-of-line)
            if matches!(
                (key_event.code, key_event.modifiers),
                (KeyCode::Char('a'), KeyModifiers::CONTROL)
            ) {
                state.editor.textarea.select_all();
            // Down arrow: move down then jump to end of that line
            } else if matches!(key_event.code, KeyCode::Down) {
                state.editor.textarea.input(key_event);
                state.editor.textarea.move_cursor(CursorMove::End);
            } else {
                state.editor.handle_key(key_event);
            }

            // Set debounce urgency based on what was typed
            if matches!(key_event.code, KeyCode::Enter) {
                state.debounce_duration = Duration::from_millis(DEBOUNCE_NEWLINE_MS);
                state.words_since_send += 1;
            } else if matches!(key_event.code, KeyCode::Char(' ')) {
                state.words_since_send += 1;
                let (row, col) = state.editor.textarea.cursor();
                let is_sentence_end = if let Some(line) = state.editor.textarea.lines().get(row) {
                    let chars: Vec<char> = line.chars().collect();
                    col >= 2
                        && matches!(chars.get(col.wrapping_sub(2)), Some('.' | '!' | '?'))
                } else {
                    false
                };
                // Micro-trigger: after 2-3 words, fire cleanup very fast
                state.debounce_duration = if state.words_since_send >= 3 {
                    Duration::from_millis(DEBOUNCE_MICRO_MS)
                } else if is_sentence_end {
                    Duration::from_millis(DEBOUNCE_SENTENCE_MS)
                } else {
                    Duration::from_millis(DEBOUNCE_WORD_MS)
                };
            } else {
                state.debounce_duration = Duration::from_millis(DEBOUNCE_IDLE_MS);
            }

            if state.llm_status == LlmStatus::Applied {
                state.llm_status = LlmStatus::Idle;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_modal_key(state: &mut AppState, key: KeyEvent) -> Result<()> {
    let action = keybindings::map_modal_key(key);
    match action {
        Action::Confirm => {
            state.should_quit = true;
        }
        Action::Cancel => {
            state.screen = Screen::Editor;
        }
        _ => {}
    }
    Ok(())
}

// --- Settings ---

fn handle_settings_key(state: &mut AppState, key: KeyEvent) -> Result<()> {
    let action = keybindings::map_settings_key(key);
    match action {
        Action::Tab => {
            state.settings_field = (state.settings_field + 1) % 3;
        }
        Action::Back => {
            save_settings(state);
            state.screen = Screen::Startup;
            state.transition = Some(TransitionAnim {
                start: Instant::now(),
                style: RevealStyle::ZoomOut,
            });
        }
        Action::Quit => {
            save_settings(state);
            state.should_quit = true;
        }
        Action::OpenUrl => {
            crate::config::open_provider_url(state.settings_field as usize);
        }
        Action::ForwardToEditor(key_event) => {
            let idx = state.settings_field as usize;
            state.settings_inputs[idx].input(key_event);
        }
        _ => {}
    }
    Ok(())
}

fn init_settings_inputs(state: &mut AppState) {
    let keys = crate::config::load_saved_keys();
    for (i, key) in keys.iter().enumerate() {
        state.settings_inputs[i].select_all();
        state.settings_inputs[i].cut();
        if !key.is_empty() {
            state.settings_inputs[i].insert_str(key);
        }
    }
    state.settings_field = 0;
}

fn save_settings(state: &mut AppState) {
    let keys = [
        state.settings_inputs[0].lines().join(""),
        state.settings_inputs[1].lines().join(""),
        state.settings_inputs[2].lines().join(""),
    ];
    crate::config::save_api_keys(&keys);
    state.llm_config = crate::config::load_config();
    state.llm_enabled = state.llm_config.is_some();
    state.llm_status = if state.llm_config.is_some() {
        LlmStatus::Idle
    } else {
        LlmStatus::Disabled
    };
}

fn return_to_startup(state: &mut AppState) -> Result<()> {
    // Auto-save if there's content
    if state.editor.modified && !state.editor.content().trim().is_empty() {
        save_document(state)?;
    }
    state.screen = Screen::Startup;
    state.editor = EditorState::new();
    state.page_stack.clear();
    state.current_file = PathBuf::new();
    state.current_page_name = String::new();
    state.in_flight = false;
    state.llm_status = if state.llm_config.is_some() {
        LlmStatus::Idle
    } else {
        LlmStatus::Disabled
    };
    state.transition = Some(TransitionAnim {
        start: Instant::now(),
        style: RevealStyle::ZoomOut,
    });
    Ok(())
}

// --- Graph-node navigation ---

fn navigate_into_link(state: &mut AppState) -> Result<()> {
    let link_name = match state.editor.find_link_at_cursor() {
        Some(name) => name,
        None => return Ok(()),
    };

    // Auto-save current page before navigating
    save_document_quiet(state)?;

    // Push current page onto stack
    let current_content = state.editor.content();
    let current_cursor = state.editor.textarea.cursor();
    state.page_stack.push(PageEntry {
        file_path: state.current_file.clone(),
        display_name: state.current_page_name.clone(),
        content: current_content,
        cursor: current_cursor,
        modified: state.editor.modified,
        last_sent_hash: state.editor.last_sent_hash.clone(),
    });

    // Determine linked page path: {output_dir}/{doc_title}/{link_name}.md
    let link_dir = PathBuf::from(&state.output_dir).join(&state.doc_title);
    fs::create_dir_all(&link_dir)?;
    let link_file = link_dir.join(format!("{}.md", link_name));

    // Load linked page content
    let content = if link_file.exists() {
        fs::read_to_string(&link_file).unwrap_or_default()
    } else {
        String::new()
    };

    // Replace editor with linked page
    state.editor.set_content_with_cursor(&content, 0, 0);
    state.editor.modified = false;
    state.editor.last_sent_hash = String::new();

    // Update tracking
    state.current_file = link_file;
    state.current_page_name = link_name;

    // Zoom-in animation
    state.transition = Some(TransitionAnim {
        start: Instant::now(),
        style: RevealStyle::ZoomIn,
    });

    Ok(())
}

fn navigate_back(state: &mut AppState) -> Result<()> {
    // Auto-save the nested page
    save_document_quiet(state)?;

    // Pop parent page
    let entry = match state.page_stack.pop() {
        Some(e) => e,
        None => return Ok(()),
    };

    // Restore editor state
    let (row, col) = entry.cursor;
    state.editor.set_content_with_cursor(&entry.content, row, col);
    state.editor.modified = entry.modified;
    state.editor.last_sent_hash = entry.last_sent_hash;

    // Restore file tracking
    state.current_file = entry.file_path;
    state.current_page_name = entry.display_name;

    // Zoom-out animation
    state.transition = Some(TransitionAnim {
        start: Instant::now(),
        style: RevealStyle::ZoomOut,
    });

    Ok(())
}

/// Save without updating UI flash indicators.
fn save_document_quiet(state: &AppState) -> Result<()> {
    if let Some(parent) = state.current_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&state.current_file, state.editor.content())?;
    Ok(())
}

// --- LLM ---

fn handle_llm_response(state: &mut AppState, response: LlmResponse) {
    if response.rate_limited {
        state.debounce_duration = Duration::from_millis(DEBOUNCE_RATE_LIMITED_MS);
        state.llm_status = LlmStatus::Error;
        return;
    }

    if response.cleaned_text.is_empty() {
        state.llm_status = LlmStatus::Error;
        return;
    }

    let current_hash = state.editor.content_hash();
    if response.original_hash != current_hash {
        state.llm_status = LlmStatus::Idle;
        return;
    }

    let original_len = state.editor.content().len();
    let cleaned_len = response.cleaned_text.len();
    if cleaned_len > original_len * 2 || (original_len > 10 && cleaned_len < original_len / 3) {
        state.llm_status = LlmStatus::Error;
        return;
    }

    let old_text = state.editor.content();
    let new_text = &response.cleaned_text;

    let changed = diff_positions(&old_text, new_text);
    if changed.is_empty() {
        state.editor.last_sent_hash = llm::content_hash(new_text);
        state.llm_status = LlmStatus::Applied;
        state.llm_status_flash = Some(Instant::now());
        state.debounce_duration = Duration::from_millis(DEBOUNCE_IDLE_MS);
        return;
    }

    // Apply cleaned text and start visual dissolve on changed positions
    state.editor.replace_content(new_text);
    state.editor.wrap_all_lines();
    state.editor.last_sent_hash = llm::content_hash(new_text);
    state.llm_status = LlmStatus::Applied;
    state.llm_status_flash = Some(Instant::now());
    state.debounce_duration = Duration::from_millis(DEBOUNCE_IDLE_MS);

    // Start per-character dissolve animation (render-buffer only)
    if !changed.is_empty() {
        let seed = Instant::now().elapsed().as_nanos() as u64;
        let resolve_times: Vec<u64> = changed
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let h = seed
                    .wrapping_add(i as u64)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                60 + ((h >> 33) % 340) // 60ms to 400ms
            })
            .collect();
        state.text_dissolve = Some(TextDissolve {
            changed_positions: changed,
            resolve_times,
            start: Instant::now(),
        });
    }
}

fn save_document(state: &mut AppState) -> Result<()> {
    if let Some(parent) = state.current_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&state.current_file, state.editor.content())?;

    state.editor.modified = false;
    state.just_saved = true;
    state.save_flash_until = Some(Instant::now() + Duration::from_secs(3));

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

// --- Helpers ---

fn diff_positions(old: &str, new: &str) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let max_lines = old_lines.len().max(new_lines.len());
    for row in 0..max_lines {
        let old_chars: Vec<char> = old_lines
            .get(row)
            .map(|l| l.chars().collect())
            .unwrap_or_default();
        let new_chars: Vec<char> = new_lines
            .get(row)
            .map(|l| l.chars().collect())
            .unwrap_or_default();

        let max_cols = old_chars.len().max(new_chars.len());
        for col in 0..max_cols {
            if old_chars.get(col) != new_chars.get(col) {
                positions.push((row, col));
            }
        }
    }

    positions
}
