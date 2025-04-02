use ratatui::{
    style::{Color, Style},
    text::{Line, Text},
    widgets::Paragraph,
};

pub fn style(color: Color) -> Style {
    Style::default().fg(color)
}

pub fn display_commit_metadata<'a>(metadata: &'a String) -> Paragraph<'a> {
    let mut mlines = metadata.lines();
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::styled(mlines.next().unwrap(), style(Color::Blue)));
    lines.push(Line::styled(mlines.next().unwrap(), style(Color::Green)));
    lines.push(Line::styled(mlines.next().unwrap(), style(Color::Yellow)));
    while let Some(line) = mlines.next() {
        lines.push(Line::styled(line, style(Color::default())));
    }
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text);
    paragraph
}
