use crate::config::{get_log_command_to_run, run_command, Config};

use crate::git::ref_to_commit;
use crate::input::basic_movements;
use crate::show_app;
use crate::ui::{author_name_from_commit, log_from_commit};

use git2::{Commit, DiffOptions, Repository};
use ratatui::crossterm::event::KeyEvent;
use ratatui::style::Style;
use ratatui::widgets::{ListState, StatefulWidget};

use crossterm::event::{self, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    style::Color,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};
use std::collections::HashMap;
use std::{env, io};
use std::path::Path;

pub fn log_app(
    config: &Config,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    path: Option<String>,
    rev: Option<String>,
    author_filter: Option<String>,
) -> io::Result<()> {
    // view model
    let mut state = ListState::default();
    let mut loading = true;
    state.select_first();
    // model
    let mut quit = false;
    let mut height = 0;

    let repo = Repository::open(Path::new(".")).unwrap();
    let head = repo.head().unwrap();
    let mut current_commit = match rev {
        Some(r) => ref_to_commit(&repo, Some(r)).unwrap().clone(),
        None => {
            let commit_id = head.target().unwrap();
            let commit = repo.find_commit(commit_id).unwrap();
            commit.clone()
        }
    };

    let mut max_len_author = 0;

    let mut refs_table: HashMap<String, Vec<String>> = HashMap::new();
    let refs = repo.references().unwrap();
    for reference in refs {
        let reference = reference.unwrap();
        if let Some(ref_name) = reference.shorthand() {
            let oid = reference.peel_to_commit().unwrap().id().to_string();
            refs_table
                .entry(oid)
                .or_insert_with(Vec::new)
                .push(ref_name.to_string());
        }
    }

    let mut commits: Vec<Commit> = Vec::new();
    let mut commit_items: Vec<ListItem> = Vec::new();

    // for path parameter
    let mut diff_opts = DiffOptions::new();
    if let Some(ref p) = path {
        let abs_path = Path::new(p);
        let cwd = env::current_dir().unwrap();

        match abs_path.strip_prefix(&cwd) {
            Ok(relative_path) => {
                let st = relative_path.to_str().unwrap();
                diff_opts.pathspec(st);
            },
            Err(_) => (),
        }
    }

    while !quit {
        if loading {
            let mut commits_to_add: Vec<Commit> = Vec::new();
            let mut idx = 0;
            while idx < 1000 {
                let mut filter = false;
                if path.is_some() {
                    if let Ok(parent) = current_commit.parent(0) {
                        let commit_tree = current_commit.tree().unwrap();
                        let parent_tree = parent.tree().unwrap();
                        let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), Some(&mut diff_opts)).unwrap();
                        if diff.deltas().len() > 0 {
                            filter = true;
                        }
                    }
                }
                if !filter && author_filter.is_some() {
                    let author = current_commit.author();
                    let author_name = author.name().unwrap_or_default();
                    if !author_name.to_lowercase().contains(&author_filter.clone().unwrap().to_lowercase()) {
                        filter = true;
                    }
                }
                if !filter {
                    commits_to_add.push(current_commit.clone());
                    idx += 1;
                }

                if let Some(parent) = current_commit.parent(0).ok() {
                    current_commit = parent;
                } else {
                    loading = false;
                    break;
                }
            }
            if max_len_author == 0 {
                max_len_author = commits_to_add
                    .iter()
                    .map(|c| author_name_from_commit(c).len())
                    .max()
                    .unwrap_or(0);
            }
            let mut commit_items_to_add: Vec<ListItem> = Vec::new();
            for commit in commits_to_add {
                let line = log_from_commit(commit.clone(), max_len_author, &refs_table);
                commit_items_to_add.push(ListItem::new(line));
                commits.push(commit.clone());
            }
            commit_items.extend(commit_items_to_add);
        }

        // TODO: might be sufficient for optimization ?
        let commit_list = List::new(commit_items.iter().cloned().collect::<Vec<ListItem>>())
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::new().bg(Color::Black))
            .scroll_padding(config.scroll_off);

        // ui
        terminal.draw(|f| {
            let size = f.area();
            height = size.height as usize;
            StatefulWidget::render(&commit_list, size, f.buffer_mut(), &mut state);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event::Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            else {
                continue;
            };

            if commits.len() == 0 {
                match code {
                    KeyCode::Char('q') | KeyCode::Enter => quit = true,
                    _ => (),
                }
                continue;
            }

            let commit = commits[state.selected().unwrap()].clone();

            if let Some(command) = get_log_command_to_run(&config, code) {
                let mut clear = false;

                run_command(
                    command,
                    &mut quit,
                    &mut clear,
                    None,
                    Some(commit.id().to_string()),
                );

                if clear {
                    let _ = terminal.clear()?;
                }
                continue;
            }

            if basic_movements(code, modifiers, &mut state, height, &mut quit) {
                continue;
            }

            match code {
                KeyCode::Enter => {
                    let _ = terminal.clear();
                    let commit = commits[state.selected().unwrap()].clone();
                    let _ = show_app(&config, terminal, Some(commit.id().to_string()));
                    let _ = terminal.clear();
                }
                _ => (),
            }
        }
    }
    Ok(())
}
