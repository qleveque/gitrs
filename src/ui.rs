use std::{collections::HashMap, path::Path};

use git2::{Commit, Delta};
use ratatui::{style::{Color, Style}, text::{Line, Span}};
use syntect::{easy::HighlightLines, highlighting::{Style as SyntectStyle, ThemeSet}, parsing::SyntaxSet, util::LinesWithEndings};

pub fn style(color: Color) -> Style {
    Style::default().fg(color)
}

pub fn delta_char(delta: Delta) -> char {
    match delta {
        Delta::Modified => '>',
        Delta::Deleted => '-',
        Delta::Added | Delta::Untracked => '+',
        Delta::Conflicted => '@',
        _ => ' ',
    }
}

pub fn delta_color(delta: Delta) -> Color {
    match delta {
        Delta::Added => Color::Green,
        Delta::Deleted => Color::Red,
        _ => Color::LightBlue,
    }
}

pub fn delta_value(delta: Delta) -> u8 {
    match delta {
        Delta::Added => 0,
        Delta::Modified => 1,
        Delta::Deleted => 2,
        _ => 3,
    }
}

pub fn blame_from_commit<'a>(commit: Commit, max_len_author: usize, line: usize, max_len_line: usize) -> Line<'a> {
    let author_name = author_name_from_commit(&commit);
    let date = date_from_commit(&commit);
    let spans = vec![
        Span::styled(
            format!("{:<max_len_author$}", author_name),
            style(Color::Yellow),
        ),
        Span::raw(" "),
        Span::styled(date, style(Color::Blue)),
        Span::raw(" "),
        Span::styled(
            format!("{:>max_len_line$}", line),
            style(Color::Yellow)
        ),
    ];

    Line::from(spans)
}

pub fn log_from_commit<'a>(
    commit: Commit,
    max_len_author: usize,
    refs_table: &'a HashMap<String, Vec<String>>,
) -> Line<'a> {
    let author_name = author_name_from_commit(&commit);
    let date = date_from_commit(&commit);
    let message = commit.message().unwrap_or("No message").to_string();
    let refs = refs_table.get(&commit.id().to_string());
    let title = message.lines().next().unwrap_or("No title").to_string();

    let mut spans = vec![
        Span::styled(date, style(Color::Blue)),
        Span::raw(" "),
        Span::styled(
            format!("{:<max_len_author$}", author_name),
            style(Color::Yellow),
        ),
        Span::raw(" "),
        Span::styled(title, style(Color::Gray)),
    ];
    if let Some(references) = refs {
        for reference in references {
            spans.push(Span::styled(format!(" ({})", reference), style(Color::Red)));
        }
    }
    Line::from(spans)
}

pub fn author_name_from_commit(commit: &Commit) -> String {
    commit.author().name().unwrap_or("Unknown").to_string()
}

pub fn date_from_commit(commit: &Commit) -> String {
    let seconds = commit.time().seconds();
    let commit_datetime = chrono::DateTime::from_timestamp(seconds, 0).unwrap();
    commit_datetime.format("%Y-%m-%d").to_string()
}

pub fn highlight_code<'a>(file: &'a String, content: &'a String) -> Vec<Line<'a>> {
    let extension = Path::new(file).extension();
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension(extension.unwrap_or_default().to_str().unwrap());

    let mut r: Vec<Line> = Vec::new();
    if syntax.is_none() {
        for line in LinesWithEndings::from(content) {
            r.push(Line::from(line));
        }
        return r;
    }
    let mut h = HighlightLines::new(syntax.unwrap(), &ts.themes["base16-ocean.dark"]);

    for line in LinesWithEndings::from(content) {
        let ranges: Vec<(SyntectStyle, &str)> = h.highlight_line(line, &ps).unwrap();
        let spans: Vec<Span> = ranges
            .iter()
            .map(|(style, text)| {
                Span::styled(
                    *text,
                    Style::default().fg(Color::Rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    )),
                )
            })
            .collect();
        r.push(Line::from(spans));
    }
    r
}
