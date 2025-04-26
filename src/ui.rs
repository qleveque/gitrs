use ratatui::style::{Color, Style};


pub fn highlight_style() -> Style {
     Style::from(Color::Rgb(255, 255, 255)).bg(Color::DarkGray)
}

pub fn age_to_color(age_factor: f32) -> Color {
    let clamped = age_factor.clamp(0.0, 1.0);
    let r = (255.0 * (1.0 - clamped) + 80.0 * clamped) as u8;
    let g = (255.0 * (1.0 - clamped) + 80.0 * clamped) as u8;
    let b = (200.0 * (1.0 - clamped) + 80.0 * clamped) as u8;
    Color::Rgb(r, g, b)
}
