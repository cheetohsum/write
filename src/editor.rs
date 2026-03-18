use crossterm::event::KeyEvent;
use tui_textarea::{CursorMove, TextArea};

use crate::llm::content_hash;
use crate::theme;

pub struct EditorState<'a> {
    pub textarea: TextArea<'a>,
    pub last_sent_hash: String,
    pub modified: bool,
    pub wrap_width: u16,
}

impl<'a> EditorState<'a> {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(ratatui::style::Style::default().bg(theme::PARCHMENT));
        textarea.set_cursor_style(theme::cursor());
        textarea.set_style(theme::base());
        textarea.set_selection_style(
            ratatui::style::Style::default()
                .bg(theme::WHEAT)
                .fg(theme::WALNUT),
        );
        textarea.set_line_number_style(theme::secondary());
        textarea.set_block(
            ratatui::widgets::Block::default().style(theme::base()),
        );

        EditorState {
            textarea,
            last_sent_hash: String::new(),
            modified: false,
            wrap_width: 80,
        }
    }

    pub fn content(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn content_hash(&self) -> String {
        content_hash(&self.content())
    }

    pub fn word_count(&self) -> usize {
        self.content().split_whitespace().count()
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.textarea.input(key);
        self.modified = true;
        self.wrap_current_line();
    }

    fn wrap_current_line(&mut self) {
        let max = self.wrap_width as usize;
        if max == 0 {
            return;
        }

        let (row, _col) = self.textarea.cursor();
        let line = match self.textarea.lines().get(row) {
            Some(l) => l.clone(),
            None => return,
        };

        if line.len() <= max {
            return;
        }

        // Find the last space at or before max
        let break_at = match line[..max].rfind(' ') {
            Some(pos) => pos,
            None => return, // single long word, don't break
        };

        // Move cursor to the break position
        self.textarea.move_cursor(CursorMove::Head);
        for _ in 0..break_at {
            self.textarea.move_cursor(CursorMove::Forward);
        }

        // Delete the space and insert a newline
        self.textarea.delete_next_char();
        self.textarea.insert_newline();
    }

    pub fn replace_content(&mut self, new_text: &str) {
        let (row, col) = self.textarea.cursor();

        // Select all and delete
        self.textarea.select_all();
        self.textarea.cut();

        // Insert new text
        self.textarea.insert_str(new_text);

        // Restore cursor - clamp to valid range
        let line_count = self.textarea.lines().len();
        let target_row = row.min(line_count.saturating_sub(1));
        self.textarea.move_cursor(CursorMove::Top);
        self.textarea.move_cursor(CursorMove::Head);

        for _ in 0..target_row {
            self.textarea.move_cursor(CursorMove::Down);
        }
        let line_len = self
            .textarea
            .lines()
            .get(target_row)
            .map(|l| l.len())
            .unwrap_or(0);
        let target_col = col.min(line_len);
        for _ in 0..target_col {
            self.textarea.move_cursor(CursorMove::Forward);
        }
    }
}
