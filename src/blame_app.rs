use crate::config::{get_blame_command_to_run, Config};

use crate::git::{git_blame_output, CommitRef};
use crate::input::InputManager;
use crate::show_app;
use crate::ui::display_blame_line;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListState, StatefulWidget};

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

use crossterm::event::KeyCode;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use std::io;

fn highlight_code(code: &Vec<String>) -> Vec<Line> {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension("rs").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

    code.iter()
        .map(|line| {
            let ranges: Vec<(SyntectStyle, String)> = h
                .highlight_line(&line, &ps)
                .unwrap()
                .into_iter()
                .map(|(style, text)| (style, text.to_string())) // Convert &str to owned String
                .collect();
            let spans: Vec<Span> = ranges
                .into_iter()
                .map(|(style, text)| {
                    Span::styled(
                        text, // Now owns the string
                        Style::default().fg(Color::Rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        )),
                    )
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

fn parse_git_blame(
    file: String,
    revision: Option<String>,
    config: &Config,
) -> (Vec<Option<CommitRef>>, Vec<String>) {
    let output = git_blame_output(file, revision.clone(), config);

    let mut blame_column = Vec::new();
    let mut code_column = Vec::new();

    for line in output.lines() {
        let (blame, code) = line.split_once(')').unwrap();
        code_column.push(code.to_string());
        let blame_text = blame.to_string() + ")";
        let (hash, blame_text) = blame_text.split_once(" (").unwrap();
        // for initial commit
        blame_column.push(if hash.starts_with("0000") {
            None
        } else {
            let metadata: Vec<&str> = blame_text.trim().split_whitespace().collect();
            let author = metadata[..metadata.len() - 4].join(" ");
            let date = metadata[metadata.len() - 4];
            Some(CommitRef::new(
                hash.to_string(),
                author.to_string(),
                date.to_string(),
            ))
        });
    }

    (blame_column, code_column)
}

pub fn blame_app(
    config: &Config,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    file: String,
    revision: Option<String>,
    line: usize,
) -> io::Result<()> {
    // model
    let mut blames: Vec<Option<CommitRef>> = Vec::new();
    let mut code: Vec<String>;

    let mut quit = false;
    let mut reload = true;
    let mut height = 0;

    let mut input_manager = InputManager::new();

    // view model
    let mut blame_list = List::default();
    let mut code_list = List::default();
    let mut state = ListState::default();
    let mut max_blame_len = 0;
    state.select(Some(line - 1));

    let mut revisions: Vec<Option<String>> = vec![revision];
    let mut first = true;

    while !quit {
        if reload {
            let (new_blames, new_code) =
                parse_git_blame(file.clone(), revisions.last().unwrap().clone(), &config);
            if new_blames.len() == 0 {
                revisions.pop();
                continue;
            }
            max_blame_len = 0;
            blames = new_blames;
            code = new_code;
            let len = blames.len();
            let max_author_len = blames
                .iter()
                .map(|opt_commit| match opt_commit {
                    Some(commit) => commit.author.len(),
                    _ => "Not Committed Yet".len(),
                })
                .max()
                .unwrap();
            let max_line_len = format!("{}", blames.len()).len();
            let blame_items: Vec<ListItem> = blames
                .iter()
                .enumerate()
                .map(|(idx, opt_commit)| {
                    let display = display_blame_line(
                        opt_commit,
                        idx,
                        max_author_len,
                        max_line_len,
                        &mut max_blame_len,
                    );
                    ListItem::new(display) // .style(style)
                })
                .collect();
            blame_list = List::new(blame_items)
                .block(Block::default())
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .scroll_padding(config.scroll_off);
            let code_items: Vec<ListItem> = highlight_code(&code)
                .iter()
                .map(|line| {
                    ListItem::new(line.clone()) // .style(style)
                })
                .collect();
            code_list = List::new(code_items)
                .block(Block::default().borders(Borders::LEFT))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::new().fg(Color::Black).bg(Color::Gray))
                .scroll_padding(config.scroll_off);

            match state.selected() {
                None => state.select(Some(len - 1)),
                Some(idx) => {
                    if idx >= len {
                        state.select(Some(len - 1));
                    }
                }
            }
            reload = false;
        }

        // ui
        terminal.draw(|f| {
            let size = f.area();
            height = size.height as usize;

            if first {
                state = if state.selected().unwrap() > height / 2 {
                    let idx = state.selected().unwrap() - height / 2;
                    state.clone().with_offset(idx)
                } else {
                    state.clone().with_offset(0)
                };
                first = false;
            }

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(max_blame_len as u16), Constraint::Min(0)])
                .split(f.area());

            StatefulWidget::render(&blame_list, chunks[0], f.buffer_mut(), &mut state);

            StatefulWidget::render(&code_list, chunks[1], f.buffer_mut(), &mut state);
        })?;

        if !input_manager.key_pressed()? {
            continue;
        }
        let idx = state.selected().unwrap();
        // TODO: if first commit starts with ^
        let opt_commit = blames.get(idx).unwrap();

        let rev = match opt_commit {
            Some(commit) => Some(commit.hash.clone()),
            _ => None,
        };
        let (opt_command, potential) =
            get_blame_command_to_run(&config, input_manager.key_combination.clone());
        if input_manager.handle_generic_user_input(
            &mut state,
            height,
            &mut quit,
            opt_command,
            Some(file.clone()),
            rev,
            potential,
            terminal,
        )? {
            continue;
        }

        match input_manager.key_event.code {
            KeyCode::Char('r') => {
                reload = true;
            }
            KeyCode::Char('l') => {
                if revisions.len() == 1 {
                    continue;
                }
                revisions.pop();
                reload = true;
            }
            KeyCode::Char('h') => {
                let idx = state.selected().unwrap();
                let commit_ref = blames.get(idx).unwrap();
                let rev = if let Some(commit) = commit_ref {
                    if let Some('^') = commit.hash.chars().next() {
                        continue;
                    }
                    format!("{}^", commit.hash)
                } else {
                    "HEAD".to_string()
                };
                revisions.push(Some(rev.clone()));
                reload = true;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let idx = state.selected().unwrap();
                let commit_ref = blames.get(idx).unwrap();

                let rev = if let Some(commit) = commit_ref {
                    if commit.hash.starts_with('^') {
                        Some(commit.hash[1..].to_string())
                    } else {
                        Some(commit.hash.clone())
                    }
                } else {
                    None
                };

                let _ = terminal.clear();
                let _ = show_app(&config, terminal, rev);
                let _ = terminal.clear();
            }
            _ => (),
        }
    }
    Ok(())
}
