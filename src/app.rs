use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use crate::config::LlmConfig;
use crate::editor::EditorState;
use crate::keybindings::{self, Action};
use crate::llm::{self, LlmRequest, LlmResponse};
use crate::theme;
use crate::ui;

const DEBOUNCE_IDLE_MS: u64 = 2000;
const DEBOUNCE_WORD_MS: u64 = 800;
const DEBOUNCE_SENTENCE_MS: u64 = 300;
const DEBOUNCE_RATE_LIMITED_MS: u64 = 8000;
const ANIMATION_DURATION_MS: u64 = 200;
const STARTUP_ANIM_MS: u64 = 900;
const TRANSITION_MS: u64 = 350;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Startup,
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

pub struct AnimationState {
    pub new_text: String,
    pub changed_positions: Vec<(usize, usize)>,
    pub resolve_times: Vec<u64>,
    pub start: Instant,
}

pub struct StartupAnim {
    pub start: Instant,
}

pub struct TransitionAnim {
    pub start: Instant,
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
    pub llm_enabled: bool,
    pub just_saved: bool,
    pub save_flash_until: Option<Instant>,
    pub last_keypress: Instant,
    pub debounce_duration: Duration,
    pub in_flight: bool,
    pub should_quit: bool,
    pub animation: Option<AnimationState>,
    pub startup_anim: Option<StartupAnim>,
    pub transition: Option<TransitionAnim>,
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
            llm_enabled,
            just_saved: false,
            save_flash_until: None,
            last_keypress: Instant::now(),
            debounce_duration: Duration::from_millis(DEBOUNCE_IDLE_MS),
            in_flight: false,
            should_quit: false,
            animation: None,
            startup_anim: Some(StartupAnim {
                start: Instant::now(),
            }),
            transition: None,
        }
    }
}

pub async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    llm_config: Option<LlmConfig>,
) -> Result<()> {
    let mut state = AppState::new(llm_config.clone());

    let (llm_tx, mut llm_rx): (
        Option<mpsc::Sender<LlmRequest>>,
        Option<mpsc::Receiver<LlmResponse>>,
    ) = if let Some(ref cfg) = llm_config {
        let (tx, rx) = llm::spawn_llm_task(cfg.clone());
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    loop {
        // Update wrap width
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

        // Drive LLM cleanup animation
        let anim_action = if let Some(ref anim) = state.animation {
            let elapsed = anim.start.elapsed().as_millis() as u64;
            if elapsed >= ANIMATION_DURATION_MS {
                Some(AnimAction::Complete(anim.new_text.clone()))
            } else {
                let frame = generate_scramble_frame(
                    &anim.new_text,
                    &anim.changed_positions,
                    &anim.resolve_times,
                    elapsed,
                    anim.start.elapsed().as_nanos() as u64,
                );
                Some(AnimAction::Frame(frame))
            }
        } else {
            None
        };
        match anim_action {
            Some(AnimAction::Complete(text)) => {
                state.animation = None;
                state.editor.replace_content(&text);
                state.editor.last_sent_hash = llm::content_hash(&text);
                state.llm_status = LlmStatus::Applied;
            }
            Some(AnimAction::Frame(frame)) => {
                state.editor.replace_content(&frame);
            }
            None => {}
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

        // Check debounce: should we send to LLM?
        if state.screen == Screen::Editor
            && state.llm_enabled
            && !state.in_flight
            && state.editor.modified
            && state.animation.is_none()
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
                        state.llm_status = LlmStatus::Cleaning;
                        let _ = tx.try_send(LlmRequest {
                            text,
                            hash: current_hash,
                        });
                    }
                }
            }
        }

        // Poll rate: fast during any animation
        let animating = state.animation.is_some()
            || state.startup_anim.is_some()
            || state.transition.is_some();
        let poll_duration = if animating {
            Duration::from_millis(30)
        } else {
            Duration::from_millis(100)
        };

        if event::poll(poll_duration)? {
            if let Event::Key(key) = event::read()? {
                // During startup animation, only allow quit
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

                // During transition, swallow all input
                if state.transition.is_some() {
                    continue;
                }

                state.last_keypress = Instant::now();

                // Cancel LLM animation on keypress
                if let Some(anim) = state.animation.take() {
                    state.editor.replace_content(&anim.new_text);
                    state.editor.last_sent_hash = llm::content_hash(&anim.new_text);
                    state.llm_status = LlmStatus::Applied;
                }

                handle_key(&mut state, key)?;

                if state.should_quit {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn handle_key(state: &mut AppState, key: KeyEvent) -> Result<()> {
    match state.screen {
        Screen::Startup => handle_startup_key(state, key),
        Screen::Editor => handle_editor_key(state, key),
        Screen::QuitConfirm => handle_modal_key(state, key),
    }
}

fn handle_startup_key(state: &mut AppState, key: KeyEvent) -> Result<()> {
    let action = keybindings::map_startup_key(key);
    match action {
        Action::Tab => {
            state.startup_field = if state.startup_field == 0 { 1 } else { 0 };
        }
        Action::Confirm => {
            let dir = state.dir_input.lines().join("");
            let title = state.title_input.lines().join("");

            if title.trim().is_empty() {
                return Ok(());
            }

            state.output_dir = dir;
            state.doc_title = sanitize_filename(&title);
            // Switch to editor and start transition animation
            state.screen = Screen::Editor;
            state.transition = Some(TransitionAnim {
                start: Instant::now(),
            });
        }
        Action::Quit => {
            state.should_quit = true;
        }
        Action::ForwardToEditor(key_event) => {
            if state.startup_field == 0 {
                state.dir_input.input(key_event);
            } else {
                state.title_input.input(key_event);
            }
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
        Action::ForwardToEditor(key_event) => {
            state.editor.handle_key(key_event);

            // Adjust debounce based on what was typed
            if matches!(key_event.code, KeyCode::Char(' ')) {
                let (row, col) = state.editor.textarea.cursor();
                let is_sentence_end = if let Some(line) = state.editor.textarea.lines().get(row) {
                    let chars: Vec<char> = line.chars().collect();
                    col >= 2
                        && matches!(chars.get(col.wrapping_sub(2)), Some('.' | '!' | '?'))
                } else {
                    false
                };
                state.debounce_duration = if is_sentence_end {
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
        state.debounce_duration = Duration::from_millis(DEBOUNCE_IDLE_MS);
        return;
    }

    let start_nanos = Instant::now().elapsed().as_nanos() as u64;
    let resolve_times: Vec<u64> = changed
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let seed = start_nanos
                .wrapping_add(i as u64)
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            30 + ((seed >> 33) % 160)
        })
        .collect();

    state.animation = Some(AnimationState {
        new_text: new_text.clone(),
        changed_positions: changed,
        resolve_times,
        start: Instant::now(),
    });
    state.llm_status = LlmStatus::Cleaning;
    state.debounce_duration = Duration::from_millis(DEBOUNCE_IDLE_MS);
}

fn save_document(state: &mut AppState) -> Result<()> {
    let dir = PathBuf::from(&state.output_dir);
    fs::create_dir_all(&dir)?;

    let filename = format!("{}.md", state.doc_title);
    let path = dir.join(filename);
    fs::write(&path, state.editor.content())?;

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

// --- Animation helpers ---

enum AnimAction {
    Frame(String),
    Complete(String),
}

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

fn generate_scramble_frame(
    new_text: &str,
    changed_positions: &[(usize, usize)],
    resolve_times: &[u64],
    elapsed_ms: u64,
    frame_seed: u64,
) -> String {
    let mut lines: Vec<Vec<char>> = new_text.lines().map(|l| l.chars().collect()).collect();
    if lines.is_empty() {
        lines.push(Vec::new());
    }

    for (i, &(row, col)) in changed_positions.iter().enumerate() {
        if elapsed_ms < resolve_times[i] {
            if let Some(line) = lines.get_mut(row) {
                if col < line.len() {
                    let seed = frame_seed
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(i as u64 * 2654435761)
                        .wrapping_add(elapsed_ms / 35);
                    line[col] = pseudo_random_char(seed);
                }
            }
        }
    }

    lines
        .iter()
        .map(|l| l.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn pseudo_random_char(seed: u64) -> char {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let h = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    CHARS[((h >> 33) as usize) % CHARS.len()] as char
}
