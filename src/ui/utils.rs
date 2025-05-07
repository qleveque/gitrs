use crate::model::{action::Action, app_state::NotifChannel, config::Button};
use chrono::{NaiveDate, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Clear, Paragraph, Widget},
    Frame,
};
use std::collections::HashMap;

pub const SPINNER_FRAMES: &[char] = &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];

pub fn highlight_style() -> Style {
    Style::from(Color::Rgb(255, 255, 255)).bg(Color::DarkGray)
}

pub fn search_highlight_style() -> Style {
    Style::from(Color::DarkGray)
        .bg(Color::Rgb(255, 255, 0))
        .add_modifier(Modifier::REVERSED)
}

pub fn bar_style() -> Style {
    Style::default().bg(Color::Rgb(25, 25, 25))
}

pub fn button_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn hovered_button_style() -> Style {
    Style::default()
        .bg(Color::LightBlue)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub fn clicked_button_style() -> Style {
    Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::REVERSED | Modifier::BOLD)
}

pub fn date_to_color(date: &str) -> Color {
    let today = Utc::now().date_naive();
    let past_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap_or(today);
    let age_factor = (today - past_date).num_days() as f32 / (365.0 * 2.0);

    let clamped = age_factor.clamp(0.0, 1.0);
    let r = (255.0 * (1.0 - clamped) + 80.0 * clamped) as u8;
    let g = (255.0 * (1.0 - clamped) + 80.0 * clamped) as u8;
    let b = (200.0 * (1.0 - clamped) + 80.0 * clamped) as u8;
    Color::Rgb(r, g, b)
}

pub fn clean_buggy_characters(line: &str) -> String {
    line.replace("\t", "    ").replace("\r", "^M")
}

pub fn display_edit_bar(
    edit_string: &str,
    prefix: &str,
    cursor: usize,
    chunk: &mut Rect,
    frame: &mut Frame,
) -> Rect {
    let mut displayed_string = edit_string.to_string();
    displayed_string.push(' ');
    let chars: Vec<char> = displayed_string.chars().collect();
    let before: String = chars[..cursor].iter().collect();
    let middle: String = chars[cursor..cursor + 1].iter().collect();
    let after: String = chars[cursor + 1..].iter().collect();

    let spans = vec![
        Span::styled(prefix.to_string(), Style::from(Color::Blue)),
        Span::raw(before),
        Span::styled(middle, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(after),
    ];
    let line = Line::from(spans);

    let paragraph = Paragraph::new(line);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(*chunk);
    frame.render_widget(Clear, chunks[1]);
    Widget::render(&paragraph, chunks[1], frame.buffer_mut());

    *chunk = chunks[0];
    chunks[1]
}

pub fn display_notifications(
    notifications: &HashMap<NotifChannel, String>,
    loading_char: char,
    loaded: bool,
    chunk: &mut Rect,
    frame: &mut Frame,
) {
    if notifications.is_empty() {
        return;
    }
    let mut notif_vec: Vec<_> = notifications.iter().collect();
    notif_vec.sort_by_key(|(notif_channel, _)| *notif_channel);

    let lines: Vec<Line> = notif_vec
        .into_iter()
        .map(|(notif_channel, message)| {
            let line_style = match notif_channel {
                NotifChannel::Error => Style::from(Color::Red),
                _ => Style::from(Color::Blue),
            };
            let mut message = message.clone();
            match notif_channel {
                NotifChannel::Search => {
                    message.push(' ');
                    message.push(loading_char);
                }
                NotifChannel::Line if !loaded => {
                    message.push_str("... ");
                    message.push(loading_char);
                }
                _ => (),
            };
            Line::styled(message.to_string(), line_style)
        })
        .collect();
    let paragraph = Paragraph::new(Text::from(lines)).style(bar_style());

    let len = notifications.len() as u16;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(len)])
        .split(*chunk);
    frame.render_widget(Clear, chunks[1]);
    Widget::render(&paragraph, chunks[1], frame.buffer_mut());
    *chunk = chunks[0];
}

pub fn display_menu_bar(
    buttons: &Vec<Button>,
    mouse_position: Position,
    mouse_down: bool,
    chunk: &mut Rect,
    frame: &mut Frame,
) -> Vec<(Rect, Action)> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(*chunk);

    let mut constraints = vec![Constraint::Length(1)];
    for button in buttons {
        constraints.push(Constraint::Length(button.0.chars().count() as u16));
        constraints.push(Constraint::Length(1));
    }

    let horizontal_chunks = Layout::default()
        .constraints(constraints)
        .direction(Direction::Horizontal)
        .split(chunks[0]);

    let paragraph = Paragraph::default().style(bar_style());
    Widget::render(&paragraph, chunks[0], frame.buffer_mut());

    let mut region_to_action = Vec::new();

    for (idx, button) in buttons.iter().enumerate() {
        let chunk = horizontal_chunks[2 * idx + 1];
        let style = if chunk.contains(mouse_position) {
            if mouse_down {
                clicked_button_style()
            } else {
                hovered_button_style()
            }
        } else {
            button_style()
        };
        let paragraph = Paragraph::new(button.0.to_string()).style(style);
        Widget::render(&paragraph, chunk, frame.buffer_mut());
        region_to_action.push((chunk, button.1.clone()))
    }
    *chunk = chunks[1];
    region_to_action
}
