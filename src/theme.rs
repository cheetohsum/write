#![allow(dead_code)]
use ratatui::style::{Color, Modifier, Style};

// Taliesin (Frank Lloyd Wright) color palette
pub const PARCHMENT: Color = Color::Rgb(245, 240, 232);
pub const WALNUT: Color = Color::Rgb(62, 47, 36);
pub const CLAY: Color = Color::Rgb(126, 105, 88);
pub const SANDSTONE: Color = Color::Rgb(189, 174, 157);
pub const TERRACOTTA: Color = Color::Rgb(183, 96, 64);
pub const SAGE: Color = Color::Rgb(127, 148, 116);
pub const GOLD: Color = Color::Rgb(196, 166, 97);
pub const UMBER: Color = Color::Rgb(92, 74, 60);
pub const CREAM: Color = Color::Rgb(250, 245, 235);
pub const WHEAT: Color = Color::Rgb(235, 222, 200);
pub const MAROON: Color = Color::Rgb(120, 38, 38);

pub fn base() -> Style {
    Style::default().fg(WALNUT).bg(PARCHMENT)
}

pub fn secondary() -> Style {
    Style::default().fg(CLAY).bg(PARCHMENT)
}

pub fn status_bar() -> Style {
    Style::default().fg(CREAM).bg(UMBER)
}

pub fn accent() -> Style {
    Style::default().fg(TERRACOTTA).bg(PARCHMENT)
}

pub fn accent_strong() -> Style {
    Style::default().fg(MAROON).bg(PARCHMENT)
}

pub fn title_bar() -> Style {
    Style::default()
        .fg(CREAM)
        .bg(UMBER)
        .add_modifier(Modifier::BOLD)
}

pub fn border() -> Style {
    Style::default().fg(SANDSTONE).bg(PARCHMENT)
}

pub fn border_active() -> Style {
    Style::default().fg(MAROON).bg(PARCHMENT)
}

pub fn input_active() -> Style {
    Style::default().fg(WALNUT).bg(PARCHMENT)
}

pub fn input_inactive() -> Style {
    Style::default().fg(CLAY).bg(PARCHMENT)
}

pub fn label() -> Style {
    Style::default().fg(WALNUT).bg(PARCHMENT)
}

pub fn hint() -> Style {
    Style::default().fg(SANDSTONE).bg(PARCHMENT)
}

pub fn modal_bg() -> Style {
    Style::default().fg(WALNUT).bg(WHEAT)
}

pub fn button() -> Style {
    Style::default()
        .fg(CREAM)
        .bg(TERRACOTTA)
        .add_modifier(Modifier::BOLD)
}

pub fn cursor() -> Style {
    Style::default().fg(PARCHMENT).bg(WALNUT)
}

pub fn decorative_line() -> Style {
    Style::default().fg(GOLD).bg(PARCHMENT)
}

pub fn decorative_line_subtle() -> Style {
    Style::default().fg(SANDSTONE).bg(PARCHMENT)
}
