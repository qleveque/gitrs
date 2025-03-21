use crate::config::{get_blame_command_to_run, run_command, Config};

use crate::git::{git_blame_output, ref_to_commit};
use crate::input::basic_movements;
use crate::show_app;
use crate::ui::{author_name_from_commit, blame_from_commit, highlight_code};

use git2::{Commit, Repository};
use ratatui::crossterm::event::KeyEvent;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{ListState, StatefulWidget};

use crossterm::event::{self, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use std::io;
use std::process::exit;

fn parse_git_blame(file: String, revision: Option<String>, config: &Config) -> (Vec<Option<String>>, Vec<String>) {
    let output = git_blame_output(file, revision.clone(), &config);

    let mut blame_column = Vec::new();
    let mut code_column = Vec::new();

    for line in output.lines() {
        let (blame, code) = line.split_once(')').unwrap();
        code_column.push(code.to_string());
        let (hash, _) = blame.split_once(" ").unwrap();
        // for initial commit
        blame_column.push(if hash == "00000000" {
            None
        } else {
            let h = if hash.starts_with('^') {
                // Return the string excluding the first character
                &hash[1..]
            } else {
                hash
            }
            .to_string();
            Some(h)
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
    let mut content: String;
    let mut refs: Vec<Option<String>> = Vec::new();

    let mut quit = false;
    let mut reload = true;
    let mut height = 0;

    // view model
    let mut blame_list = List::default();
    let mut code_list = List::default();
    let mut state = ListState::default();
    let mut blame_len = 0;
    state.select(Some(line - 1));

    let mut revisions: Vec<Option<String>> = vec![revision];

    let repo = Repository::open(".").unwrap();

    while !quit {
        if reload {
            let (new_refs, new_code) =
                parse_git_blame(file.clone(), revisions.last().unwrap().clone(), &config);
            if new_code.len() == 0 {
                revisions.pop();
                if revisions.len() == 0 {
                    exit(0);
                }
            } else {
                blame_len = "Not Committed Yet".len();
                refs = new_refs;
                content = new_code.join("\n").clone();

                let commits: Vec<Option<Commit>> = refs
                    .iter()
                    .map(|commit_ref| ref_to_commit(&repo, commit_ref.clone()))
                    .collect();

                let max_len_author = commits
                    .iter()
                    .filter_map(|c| c.as_ref().map(|c| author_name_from_commit(c).len()))
                    .max()
                    .unwrap_or(0);

                let max_len_line = format!("{}", refs.len()).len();
                let blame_items: Vec<ListItem> = commits
                    .iter()
                    .enumerate()
                    .map(|(idx, commit)| {
                        let display = match commit {
                            Some(commit) => {
                                let display = blame_from_commit(commit.clone(), max_len_author, idx + 1, max_len_line);
                                if blame_len < display.width() {
                                    blame_len = display.width()
                                }
                                display
                            },
                            None => Line::from("Not Committed Yet".to_string()),
                        };
                        ListItem::new(display) // .style(style)
                    })
                    .collect();

                blame_list = List::new(blame_items)
                    .block(Block::default())
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::new().bg(Color::Black))
                    .scroll_padding(config.scroll_off);
                let code_items: Vec<ListItem> = highlight_code(&file, &content)
                    .iter()
                    .map(|line| {
                        ListItem::new(line.clone()) // .style(style)
                    })
                    .collect();
                code_list = List::new(code_items)
                    .block(Block::default().borders(Borders::LEFT))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::new().bg(Color::Black))
                    .scroll_padding(config.scroll_off);

                match state.selected() {
                    None => state.select(Some(1)),
                    Some(idx) => {
                        if idx >= refs.len() {
                            state.select(Some(refs.len() - 1));
                        }
                    }
                }
            }
            reload = false;
        }

        // ui
        terminal.draw(|f| {
            let size = f.area();
            height = size.height as usize;

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(blame_len as u16 + 1),
                    Constraint::Min(0),
                ])
                .split(f.area());

            StatefulWidget::render(&blame_list, chunks[0], f.buffer_mut(), &mut state);

            StatefulWidget::render(&code_list, chunks[1], f.buffer_mut(), &mut state);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event::Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            else {
                continue;
            };

            if let Some(command) = get_blame_command_to_run(&config, code) {
                let mut clear = false;
                let idx = state.selected().unwrap();
                let commit_ref = refs.get(idx).unwrap();
                let c = ref_to_commit(
                    &repo,
                    match commit_ref {
                        Some(r) => Some(format!("{}^", r)),
                        _ => Some("HEAD".to_string()),
                    },
                );
                let hash = match c {
                    Some(commit) => Some(commit.id().to_string()),
                    _ => None,
                };

                run_command(command, &mut quit, &mut clear, Some(file.clone()), hash);

                if clear {
                    let _ = terminal.clear()?;
                }
                continue;
            }

            if basic_movements(code, modifiers, &mut state, height, &mut quit) {
                continue;
            }

            match code {
                KeyCode::Char('r') => {
                    reload = true;
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    if revisions.len() == 1 {
                        continue;
                    }
                    revisions.pop();
                    reload = true;
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    let idx = state.selected().unwrap();
                    let commit_ref = refs.get(idx).unwrap();
                    let c = ref_to_commit(
                        &repo,
                        match commit_ref {
                            Some(r) => Some(format!("{}^", r)),
                            _ => Some("HEAD".to_string()),
                        },
                    );
                    // check commit still have lines
                    if let Some(commit) = c {
                        revisions.push(Some(commit.id().to_string()));
                        reload = true;
                    }
                }
                KeyCode::Enter => {
                    let idx = state.selected().unwrap();
                    let commit_ref = refs.get(idx).unwrap();

                    let c = ref_to_commit(&repo, commit_ref.clone());
                    if let Some(commit) = c {
                        let rev = Some(commit.id().to_string());
                        let _ = terminal.clear();
                        let _ = show_app(&config, terminal, rev);
                        let _ = terminal.clear();
                    }
                }
                _ => (),
            }
        }
    }
    Ok(())
}
