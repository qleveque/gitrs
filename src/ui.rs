use chrono::{NaiveDate, Utc};
use ratatui::style::{Color, Modifier, Style};

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
    let past_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").unwrap_or(today.clone());
    let age_factor = (today - past_date).num_days() as f32 / (365.0 * 2.0);

    let clamped = age_factor.clamp(0.0, 1.0);
    let r = (255.0 * (1.0 - clamped) + 80.0 * clamped) as u8;
    let g = (255.0 * (1.0 - clamped) + 80.0 * clamped) as u8;
    let b = (200.0 * (1.0 - clamped) + 80.0 * clamped) as u8;
    Color::Rgb(r, g, b)
}

pub fn clean_buggy_characters(line: &String) -> String {
    line.replace("\t", "    ").replace("\r", "^M")
}
