use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent};
use std::io;
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

const DEBOUNCE_SECS: u64 = 4;
const DEBOUNCE_RATE_LIMITED_SECS: u64 = 8;

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

pub struct AppState<'a> {
    pub screen: Screen,
    pub llm_config: Option<LlmConfig>,
    pub editor: EditorState<'a>,
    pub doc_title: String,
    pub output_dir: String,
    pub dir_input: TextArea<'a>,
    pub title_input: TextArea<'a>,
    pub startup_field: u8, // 0 = dir, 1 = title
    pub llm_status: LlmStatus,
    pub llm_enabled: bool,
    pub just_saved: bool,
    pub save_flash_until: Option<Instant>,
    pub last_keypress: Instant,
    pub debounce_duration: Duration,
    pub in_flight: bool,
    pub should_quit: bool,
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
            startup_field: 1, // start on title field since dir has default
            llm_status,
            llm_enabled,
            just_saved: false,
            save_flash_until: None,
            last_keypress: Instant::now(),
            debounce_duration: Duration::from_secs(DEBOUNCE_SECS),
            in_flight: false,
            should_quit: false,
        }
    }
}

pub async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, llm_config: Option<LlmConfig>) -> Result<()> {
    let mut state = AppState::new(llm_config.clone());

    // Set up LLM channels if configured
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
        // Render
        terminal.draw(|f| ui::render(f, &mut state))?;

        // Handle save flash timeout
        if let Some(until) = state.save_flash_until {
            if Instant::now() > until {
                state.just_saved = false;
                state.save_flash_until = None;
            }
        }

        // Handle applied status timeout (show "applied" for 3 seconds)
        if state.llm_status == LlmStatus::Applied {
            // We'll clear it on next debounce check
        }

        // Check for LLM responses (non-blocking)
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
        {
            let elapsed = state.last_keypress.elapsed();
            if elapsed >= state.debounce_duration {
                let current_hash = state.editor.content_hash();
                if current_hash != state.editor.last_sent_hash && !state.editor.content().trim().is_empty() {
                    // Send to LLM
                    if let Some(ref tx) = llm_tx {
                        let text = state.editor.content();
                        state.editor.last_sent_hash = current_hash.clone();
                        state.in_flight = true;
                        state.llm_status = LlmStatus::Cleaning;
                        let _ = tx
                            .try_send(LlmRequest {
                                text,
                                hash: current_hash,
                            });
                    }
                }
            }
        }

        // Poll for events with short timeout for responsiveness
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                state.last_keypress = Instant::now();
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
            // Extract values and transition to editor
            let dir = state.dir_input.lines().join("");
            let title = state.title_input.lines().join("");

            if title.trim().is_empty() {
                // Don't proceed without a title
                return Ok(());
            }

            state.output_dir = dir;
            state.doc_title = sanitize_filename(&title);
            state.screen = Screen::Editor;
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
            // Reset applied status when user types
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
        state.debounce_duration = Duration::from_secs(DEBOUNCE_RATE_LIMITED_SECS);
        state.llm_status = LlmStatus::Error;
        return;
    }

    if response.cleaned_text.is_empty() {
        state.llm_status = LlmStatus::Error;
        return;
    }

    // Check staleness
    let current_hash = state.editor.content_hash();
    if response.original_hash != current_hash {
        // Content changed since we sent - discard, debounce will re-trigger
        state.llm_status = LlmStatus::Idle;
        return;
    }

    // Sanity check: response shouldn't be wildly different in length
    let original_len = state.editor.content().len();
    let cleaned_len = response.cleaned_text.len();
    if cleaned_len > original_len * 2 || cleaned_len < original_len / 3 {
        state.llm_status = LlmStatus::Error;
        return;
    }

    // Apply the cleaned text
    state.editor.replace_content(&response.cleaned_text);
    state.editor.last_sent_hash = llm::content_hash(&response.cleaned_text);
    state.llm_status = LlmStatus::Applied;

    // Reset debounce duration (in case it was doubled from rate limit)
    state.debounce_duration = Duration::from_secs(DEBOUNCE_SECS);
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
