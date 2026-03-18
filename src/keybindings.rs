use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Save,
    Quit,
    ToggleLlm,
    Confirm,
    Cancel,
    Tab,
    ForwardToEditor(KeyEvent),
}

pub fn map_editor_key(key: KeyEvent) -> Action {
    match key {
        KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => Action::Save,
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => Action::Quit,
        KeyEvent {
            code: KeyCode::Esc, ..
        } => Action::Quit,
        KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => Action::ToggleLlm,
        _ => Action::ForwardToEditor(key),
    }
}

pub fn map_startup_key(key: KeyEvent) -> Action {
    match key {
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => Action::Confirm,
        KeyEvent {
            code: KeyCode::Tab, ..
        } => Action::Tab,
        KeyEvent {
            code: KeyCode::Esc, ..
        } => Action::Quit,
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => Action::Quit,
        _ => Action::ForwardToEditor(key),
    }
}

pub fn map_modal_key(key: KeyEvent) -> Action {
    match key {
        KeyEvent {
            code: KeyCode::Char('y'),
            ..
        }
        | KeyEvent {
            code: KeyCode::Enter,
            ..
        } => Action::Confirm,
        KeyEvent {
            code: KeyCode::Char('n'),
            ..
        }
        | KeyEvent {
            code: KeyCode::Esc, ..
        } => Action::Cancel,
        _ => Action::Cancel,
    }
}
