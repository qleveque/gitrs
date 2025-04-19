use crate::action::Action;
use crate::app::GitApp;
use crate::config::Config;

use crate::git::{git_blame_output, CommitRef};
use crate::show_app::ShowApp;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListState, StatefulWidget};

use ratatui::Frame;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use std::io::{self, ErrorKind};
use std::path::Path;

pub struct BlameApp<'a> {
    file: String,
    blames: Vec<Option<CommitRef>>,
    code: Vec<String>,
    height: usize,
    blame_list: List<'a>,
    code_list: List<'a>,
    state: ListState,
    max_blame_len: usize,
    revisions: Vec<Option<String>>,
    first: bool,
    config: &'a Config,
}

impl<'a> BlameApp<'a> {
    pub fn new(
        config: &'a Config,
        file: String,
        revision: Option<String>,
        line: usize,
    ) -> Result<Self, io::Error> {
        if !Path::new(&file).exists() {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!("error: file '{}' does not exist", file),
            ));
        }
        let mut state = ListState::default();
        state.select(Some(line - 1));
        let revisions = vec![revision];
        let mut instance = Self {
            file,
            blames: Vec::new(),
            code: Vec::new(),
            height: 0,
            blame_list: List::default(),
            code_list: List::default(),
            state,
            max_blame_len: 0,
            revisions,
            first: true,
            config,
        };
        instance.reload();
        Ok(instance)
    }

    fn highlighted_lines(&self) -> Vec<Line<'a>> {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let syntax = ps.find_syntax_by_extension("rs").unwrap();
        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

        self.code
            .iter()
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

    fn displayed_blame_line(
        opt_commit: &Option<CommitRef>,
        idx: usize,
        max_author_len: usize,
        max_line_len: usize,
    ) -> Line<'a> {
        match opt_commit {
            Some(commit) => {
                let displayed_hash: String = commit.hash.chars().take(4).collect();
                let spans = vec![
                    Span::styled(displayed_hash, Style::from(Color::Blue)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:<max_author_len$}", commit.author.clone()),
                        Style::from(Color::Yellow),
                    ),
                    Span::raw(" "),
                    Span::styled(commit.date.clone(), Style::from(Color::Blue)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:>max_line_len$}", idx),
                        Style::from(Color::Yellow),
                    ),
                ];
                let line = Line::from(spans);
                line
            }
            _ => Line::from("Not Committed Yet".to_string()),
        }
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
}

impl GitApp for BlameApp<'_> {
    fn reload(&mut self) {
        let (new_blames, new_code) = BlameApp::parse_git_blame(
            self.file.clone(),
            self.revisions.last().unwrap().clone(),
            &self.config,
        );
        if new_blames.len() == 0 {
            self.revisions.pop();
            return;
        }
        self.blames = new_blames;
        self.code = new_code;
        let len = self.blames.len();
        let max_author_len = self
            .blames
            .iter()
            .map(|opt_commit| match opt_commit {
                Some(commit) => commit.author.len(),
                _ => "Not Committed Yet".len(),
            })
            .max()
            .unwrap();
        let max_line_len = format!("{}", self.blames.len()).len();

        let mut max_blame_len = 0;
        let blame_items: Vec<ListItem> = self
            .blames
            .iter()
            .enumerate()
            .map(|(idx, opt_commit)| {
                let display =
                    BlameApp::displayed_blame_line(opt_commit, idx, max_author_len, max_line_len);
                max_blame_len = max_blame_len.max(display.width());
                ListItem::new(display)
            })
            .collect();
        self.max_blame_len = max_blame_len;

        self.blame_list = List::new(blame_items)
            .block(Block::default())
            .style(Style::from(Color::White))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .scroll_padding(self.config.scroll_off);

        let code_items: Vec<ListItem> = self
            .highlighted_lines()
            .iter()
            .map(|line| ListItem::new(line.clone()))
            .collect();
        self.code_list = List::new(code_items)
            .block(Block::default().borders(Borders::LEFT))
            .style(Style::from(Color::White))
            .highlight_style(Style::from(Color::Black).bg(Color::Gray))
            .scroll_padding(self.config.scroll_off);

        match self.state.selected() {
            None => self.state.select(Some(len - 1)),
            Some(idx) => {
                if idx >= len {
                    self.state.select(Some(len - 1));
                }
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();
        self.height = size.height as usize;

        if self.first {
            self.state = if self.state.selected().unwrap() > self.height / 2 {
                let idx = self.state.selected().unwrap() - self.height / 2;
                self.state.clone().with_offset(idx)
            } else {
                self.state.clone().with_offset(0)
            };
            self.first = false;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.max_blame_len as u16),
                Constraint::Min(0),
            ])
            .split(frame.area());

        StatefulWidget::render(
            &self.blame_list,
            chunks[0],
            frame.buffer_mut(),
            &mut self.state,
        );

        StatefulWidget::render(
            &self.code_list,
            chunks[1],
            frame.buffer_mut(),
            &mut self.state,
        );
    }

    fn get_config_fields(&mut self) -> Vec<(&str, bool)> {
        vec![("blame", true)]
    }

    fn get_file_and_rev(&self) -> (Option<String>, Option<String>) {
        let idx = self.state.selected().unwrap();
        // TODO: if first commit starts with ^
        let opt_commit = self.blames.get(idx).unwrap();
        let rev = match opt_commit {
            Some(commit) => Some(commit.hash.clone()),
            _ => None,
        };
        (Some(self.file.clone()), rev)
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> bool {
        match action {
            Action::NextCommitBlame => {
                if self.revisions.len() == 1 {
                    return false;
                }
                self.revisions.pop();
                self.reload();
            }
            Action::PreviousCommitBlame => {
                let idx = self.state.selected().unwrap();
                let commit_ref = self.blames.get(idx).unwrap();
                let rev = if let Some(commit) = commit_ref {
                    if let Some('^') = commit.hash.chars().next() {
                        return false;
                    }
                    format!("{}^", commit.hash)
                } else {
                    "HEAD".to_string()
                };
                self.revisions.push(Some(rev.clone()));
                self.reload();
            }
            Action::ShowCommit => {
                let idx = self.state.selected().unwrap();
                let commit_ref = self.blames.get(idx).unwrap();

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
                let mut app = ShowApp::new(&self.config, rev);
                let _ = app.run(terminal, self.config);
                let _ = terminal.clear();
            }
            _ => {
                let mut new_state = self.state.clone();
                let quit =
                    self.run_generic_action(action, self.height, terminal, &mut new_state);
                self.state = new_state;
                return quit;
            }
        };
        return false;
    }
}
