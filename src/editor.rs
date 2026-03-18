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
        // Line numbers disabled for clean writing experience
        textarea.set_block(ratatui::widgets::Block::default().style(theme::base()));

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

        let break_at = match line[..max].rfind(' ') {
            Some(pos) => pos,
            None => return,
        };

        self.textarea.move_cursor(CursorMove::Head);
        for _ in 0..break_at {
            self.textarea.move_cursor(CursorMove::Forward);
        }

        self.textarea.delete_next_char();
        self.textarea.insert_newline();
    }

    /// Wrap all lines that exceed wrap_width. Called after LLM replacements.
    pub fn wrap_all_lines(&mut self) {
        let max = self.wrap_width as usize;
        if max == 0 {
            return;
        }
        // Collect the full text, wrap it in pure string form, then replace
        let content = self.content();
        let mut wrapped = String::new();
        for line in content.lines() {
            let mut remaining = line;
            while remaining.len() > max {
                let break_at = match remaining[..max].rfind(' ') {
                    Some(pos) => pos,
                    None => break, // single long word, leave it
                };
                wrapped.push_str(&remaining[..break_at]);
                wrapped.push('\n');
                remaining = &remaining[break_at + 1..];
            }
            wrapped.push_str(remaining);
            wrapped.push('\n');
        }
        // Remove trailing newline added by the loop
        if wrapped.ends_with('\n') {
            wrapped.pop();
        }
        if wrapped != content {
            let (row, _) = self.textarea.cursor();
            self.set_content_with_cursor(&wrapped, row, 0);
            // After wrapping, go to end of the line
            self.textarea.move_cursor(CursorMove::End);
        }
    }

    /// Replace content only if it actually changed.
    /// Cursor goes to end of last line (where the user is writing).
    pub fn replace_content(&mut self, new_text: &str) {
        if self.content() == new_text {
            return;
        }

        self.textarea.select_all();
        self.textarea.cut();
        self.textarea.insert_str(new_text);

        // Always go to the very end — bottom-right writing position
        self.textarea.move_cursor(CursorMove::Bottom);
        self.textarea.move_cursor(CursorMove::End);
    }

    pub fn set_content_with_cursor(&mut self, content: &str, row: usize, col: usize) {
        self.textarea.select_all();
        self.textarea.cut();
        self.textarea.insert_str(content);

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

    pub fn find_link_at_cursor(&self) -> Option<String> {
        let (row, col) = self.textarea.cursor();
        let line = self.textarea.lines().get(row)?;

        let mut pos = 0;
        while let Some(start) = line[pos..].find("[[") {
            let abs_start = pos + start;
            if let Some(end_offset) = line[abs_start + 2..].find("]]") {
                let abs_end = abs_start + 2 + end_offset;
                if col >= abs_start && col <= abs_end + 1 {
                    let name = &line[abs_start + 2..abs_end];
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
                pos = abs_end + 2;
            } else {
                break;
            }
        }
        None
    }

    pub fn create_link_at_cursor(&mut self) {
        if self.find_link_at_cursor().is_some() {
            return;
        }

        let (row, col) = self.textarea.cursor();
        let line = match self.textarea.lines().get(row) {
            Some(l) => l.clone(),
            None => return,
        };
        let chars: Vec<char> = line.chars().collect();

        if col >= chars.len() && chars.is_empty() {
            return;
        }

        let search_col = col.min(chars.len().saturating_sub(1));

        let mut start = search_col;
        while start > 0
            && (chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '\''
                || chars[start - 1] == '-')
        {
            start -= 1;
        }

        let mut end = search_col;
        while end < chars.len()
            && (chars[end].is_alphanumeric() || chars[end] == '\'' || chars[end] == '-')
        {
            end += 1;
        }

        if start == end {
            return;
        }

        let word_len = end - start;

        self.textarea.move_cursor(CursorMove::Head);
        for _ in 0..start {
            self.textarea.move_cursor(CursorMove::Forward);
        }

        self.textarea.insert_str("[[");

        for _ in 0..word_len {
            self.textarea.move_cursor(CursorMove::Forward);
        }

        self.textarea.insert_str("]]");

        self.modified = true;
    }
}
