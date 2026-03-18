use ratatui::layout::{Alignment, Constraint, Layout, Margin, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{AppState, LlmStatus, Screen};
use crate::theme;

pub fn render(f: &mut Frame, state: &mut AppState) {
    // Fill background
    let area = f.area();
    let bg = Block::default().style(theme::base());
    f.render_widget(bg, area);

    match state.screen {
        Screen::Startup => render_startup(f, state, area),
        Screen::Editor => render_editor(f, state, area),
        Screen::QuitConfirm => {
            render_editor(f, state, area);
            render_quit_modal(f, area);
        }
    }
}

fn render_startup(f: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Min(1),     // top padding
        Constraint::Length(14), // form
        Constraint::Min(1),     // bottom padding
    ])
    .split(area);

    let form_area = centered_rect(60, chunks[1]);

    let form_chunks = Layout::vertical([
        Constraint::Length(1), // title "write"
        Constraint::Length(1), // blank
        Constraint::Length(1), // dir label
        Constraint::Length(3), // dir input
        Constraint::Length(1), // title label
        Constraint::Length(3), // title input
        Constraint::Length(1), // blank
        Constraint::Length(1), // provider info
        Constraint::Length(1), // blank
        Constraint::Length(1), // instructions
    ])
    .split(form_area);

    // App title
    let title = Paragraph::new(Line::from(vec![
        Span::styled("write", theme::accent().add_modifier(Modifier::BOLD)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(title, form_chunks[0]);

    // Directory label
    let dir_label = Paragraph::new(Span::styled("Output Directory", theme::label()));
    f.render_widget(dir_label, form_chunks[2]);

    // Directory input
    let dir_border_style = if state.startup_field == 0 {
        theme::accent()
    } else {
        theme::border()
    };
    let dir_block = Block::default()
        .borders(Borders::ALL)
        .border_style(dir_border_style)
        .style(theme::base());

    if state.startup_field == 0 {
        state.dir_input.set_block(dir_block);
        state.dir_input.set_cursor_style(theme::cursor());
        state.dir_input.set_style(theme::input_active());
        f.render_widget(&state.dir_input, form_chunks[3]);
    } else {
        state.dir_input.set_block(dir_block);
        state.dir_input.set_cursor_style(ratatui::style::Style::default());
        state.dir_input.set_style(theme::input_inactive());
        f.render_widget(&state.dir_input, form_chunks[3]);
    }

    // Title label
    let title_label = Paragraph::new(Span::styled("Document Title", theme::label()));
    f.render_widget(title_label, form_chunks[4]);

    // Title input
    let title_border_style = if state.startup_field == 1 {
        theme::accent()
    } else {
        theme::border()
    };
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(title_border_style)
        .style(theme::base());

    if state.startup_field == 1 {
        state.title_input.set_block(title_block);
        state.title_input.set_cursor_style(theme::cursor());
        state.title_input.set_style(theme::input_active());
        f.render_widget(&state.title_input, form_chunks[5]);
    } else {
        state.title_input.set_block(title_block);
        state.title_input.set_cursor_style(ratatui::style::Style::default());
        state.title_input.set_style(theme::input_inactive());
        f.render_widget(&state.title_input, form_chunks[5]);
    }

    // Provider info
    let provider_text = match &state.llm_config {
        Some(cfg) => format!("LLM: {}", cfg.display()),
        None => "LLM: disabled (no API key found)".to_string(),
    };
    let provider = Paragraph::new(Span::styled(provider_text, theme::secondary()))
        .alignment(Alignment::Center);
    f.render_widget(provider, form_chunks[7]);

    // Instructions
    let instructions = Paragraph::new(Line::from(vec![
        Span::styled("Tab", theme::accent()),
        Span::styled(" switch fields  ", theme::secondary()),
        Span::styled("Enter", theme::accent()),
        Span::styled(" begin writing  ", theme::secondary()),
        Span::styled("Esc", theme::accent()),
        Span::styled(" quit", theme::secondary()),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(instructions, form_chunks[9]);
}

fn render_editor(f: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // title bar
        Constraint::Min(1),   // editor
        Constraint::Length(1), // status bar
    ])
    .split(area);

    // Title bar
    let modified_indicator = if state.editor.modified { " ● modified" } else { "" };
    let title_text = format!("  write | {}.md{}", state.doc_title, modified_indicator);
    let title_bar = Paragraph::new(Span::styled(title_text, theme::title_bar()))
        .style(theme::title_bar());
    f.render_widget(title_bar, chunks[0]);

    // Editor
    f.render_widget(&state.editor.textarea, chunks[1]);

    // Status bar
    let llm_status = match state.llm_status {
        LlmStatus::Disabled => "LLM: disabled",
        LlmStatus::Idle => "LLM: idle",
        LlmStatus::Waiting => "LLM: waiting...",
        LlmStatus::Cleaning => "LLM: cleaning...",
        LlmStatus::Applied => "LLM: applied ✓",
        LlmStatus::Error => "LLM: error",
        LlmStatus::Off => "LLM: off",
    };

    let save_status = if state.just_saved { "Saved ✓" } else { "" };

    let status_line = Line::from(vec![
        Span::styled(format!("  {} ", save_status), theme::status_bar()),
        Span::styled(
            format!("Words: {} ", state.editor.word_count()),
            theme::status_bar(),
        ),
        Span::styled(format!("│ {} ", llm_status), theme::status_bar()),
        Span::styled("│ Ctrl+S save  Ctrl+Q quit  Ctrl+L toggle LLM ", theme::status_bar()),
    ]);

    let status_bar = Paragraph::new(status_line).style(theme::status_bar());
    f.render_widget(status_bar, chunks[2]);
}

fn render_quit_modal(f: &mut Frame, area: Rect) {
    let modal_area = centered_rect_fixed(40, 7, area);
    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::accent())
        .title(" Unsaved Changes ")
        .title_style(theme::accent().add_modifier(Modifier::BOLD))
        .style(theme::modal_bg());
    f.render_widget(block, modal_area);

    let inner = modal_area.inner(Margin::new(2, 2));
    let msg_chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let msg = Paragraph::new("You have unsaved changes.")
        .style(theme::modal_bg())
        .alignment(Alignment::Center);
    f.render_widget(msg, msg_chunks[0]);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("Quit anyway? ", theme::modal_bg()),
        Span::styled("[Y]", theme::accent().add_modifier(Modifier::BOLD)),
        Span::styled("es  ", theme::modal_bg()),
        Span::styled("[N]", theme::accent().add_modifier(Modifier::BOLD)),
        Span::styled("o", theme::modal_bg()),
    ]))
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: false });
    f.render_widget(prompt, msg_chunks[2]);
}

fn centered_rect(percent_x: u16, area: Rect) -> Rect {
    let layout = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(area);
    layout[1]
}

fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let vert = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);
    let horiz = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(width),
        Constraint::Min(0),
    ])
    .split(vert[1]);
    horiz[1]
}
