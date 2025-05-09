use crate::app::{FileRevLine, GitApp};
use crate::model::{
    action::Action,
    app_state::{AppState, NotifChannel},
    config::{Config, MappingScope},
    errors::Error,
    git::{get_previous_filename, git_blame_output, CommitInBlame},
};
use crate::ui::utils::{date_to_color, highlight_style};

use two_face::re_exports::syntect;
use two_face::syntax;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, ThemeSet},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, StatefulWidget},
    Frame, Terminal,
};
use syntect::util::LinesWithEndings;

use std::path::Path;

struct BlameAppViewModel {
    blame_list: List<'static>,
    code_list: List<'static>,
    max_blame_len: usize,
    rect: Rect,
}

pub struct BlameApp {
    state: AppState,
    file: String,
    blames: Vec<Option<CommitInBlame>>,
    code: Vec<String>,
    revisions: Vec<Option<String>>,
    files: Vec<String>,
    view_model: BlameAppViewModel,
}

impl<'a> BlameApp {
    pub fn new(file: String, revision: Option<String>, line: usize) -> Result<Self, Error> {
        if !Path::new(&file).exists() {
            return Err(Error::Global(
                format!("file '{}' does not exist", file).to_string(),
            ));
        }
        let revisions = vec![revision];
        let files = vec![file.clone()];

        let mut state = AppState::new()?;
        state.list_state.select(Some(line - 1));
        let mut instance = Self {
            state,
            file,
            blames: Vec::new(),
            code: Vec::new(),
            revisions,
            files,
            view_model: BlameAppViewModel {
                blame_list: List::default(),
                code_list: List::default(),
                max_blame_len: 0,
                rect: Rect::default(),
            },
        };
        instance.reload()?;
        Ok(instance)
    }

    fn get_current_file(&self) -> Result<String, Error> {
        Ok(self
            .files
            .last()
            .ok_or_else(|| Error::Global("blame app revision stack empty".to_string()))?
            .to_string())
    }

    fn highlighted_lines(&mut self) -> Result<Vec<Line<'a>>, Error> {
        let syn_set = syntax::extra_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = &ts.themes["base16-ocean.dark"];

        let file_text = self.code.join("\n");
        let path = Path::new(&self.file);
        let syntax = path
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| syn_set.find_syntax_by_extension(ext))
            .unwrap_or_else(|| {
                syn_set.find_syntax_by_first_line(&file_text)
                    .unwrap_or_else(|| syn_set.find_syntax_plain_text())
            });
        let mut h = HighlightLines::new(syntax, theme);

        let mut lines: Vec<Line> = Vec::new();

        for line in LinesWithEndings::from(&file_text) {
            let ranges: Vec<(SyntectStyle, String)> = h
                .highlight_line(line, &syn_set)?
                .into_iter()
                .map(|(style, text)| (style, text.to_string())) // Convert &str to owned String
                .collect();
            let spans: Vec<Span> = ranges
                .into_iter()
                .map(|(style, text)| {
                    Span::styled(
                        text,
                        Style::default().fg(Color::Rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        )),
                    )
                })
                .collect();
            lines.push(Line::from(spans));
        }
        Ok(lines)
    }

    fn displayed_blame_line(
        opt_commit: &Option<CommitInBlame>,
        idx: usize,
        max_author_len: usize,
        max_line_len: usize,
    ) -> Line<'a> {
        match opt_commit {
            Some(commit) => {
                let date_color = date_to_color(&commit.date);
                let displayed_hash: String = commit.hash.chars().take(4).collect();
                let spans = vec![
                    Span::styled(displayed_hash, Style::from(Color::Blue)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:<max_author_len$}", commit.author.clone()),
                        Style::from(Color::Gray),
                    ),
                    Span::raw(" "),
                    Span::styled(commit.date.clone(), Style::from(date_color)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:>max_line_len$}", idx + 1),
                        Style::from(Color::DarkGray),
                    ),
                ];
                Line::from(spans)
            }
            _ => Line::from("Not Committed Yet".to_string()),
        }
    }

    fn parse_git_blame(
        file: String,
        revision: Option<String>,
        config: &Config,
    ) -> Result<(Vec<Option<CommitInBlame>>, Vec<String>), Error> {
        let output = git_blame_output(file, revision.clone(), config)?;

        let mut blame_column = Vec::new();
        let mut code_column = Vec::new();

        for line in output.lines() {
            let (blame, code) = line.split_once(')').ok_or_else(|| Error::GitParsing)?;
            code_column.push(code.to_string());
            let blame_text = blame.to_string() + ")";
            let (hash, _) = blame_text
                .split_once(" ")
                .ok_or_else(|| Error::GitParsing)?;
            // for initial commit
            blame_column.push(if hash.starts_with("0000") {
                None
            } else {
                let (_, blame_text) = blame_text
                    .split_once(" (")
                    .ok_or_else(|| Error::GitParsing)?;
                let metadata: Vec<&str> = blame_text.split_whitespace().collect();
                let author = metadata[..metadata.len() - 4].join(" ").to_string();
                let date = metadata[metadata.len() - 4].to_string();
                Some(CommitInBlame {
                    hash: hash.to_string(),
                    author,
                    date,
                })
            });
        }

        Ok((blame_column, code_column))
    }
}

impl GitApp for BlameApp {
    fn state(&mut self) -> &mut AppState {
        &mut self.state
    }

    fn get_state(&self) -> &AppState {
        &self.state
    }

    fn get_text_line(&self, idx: usize) -> Option<String> {
        self.code.get(idx).cloned()
    }

    fn reload(&mut self) -> Result<(), Error> {
        let revision = self
            .revisions
            .last()
            .ok_or_else(|| Error::Global("blame app revision stack empty".to_string()))?;
        let file = self.get_current_file()?;

        let (new_blames, new_code) =
            BlameApp::parse_git_blame(file.clone(), revision.clone(), &self.state.config)?;
        if new_blames.is_empty() {
            self.revisions.pop();
            self.files.pop();
            return Ok(());
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
            .unwrap_or(0);
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
        self.view_model.max_blame_len = max_blame_len;

        self.view_model.blame_list = List::new(blame_items)
            .highlight_style(highlight_style())
            .scroll_padding(self.state.config.scrolloff);

        let code_items: Vec<ListItem> = self
            .highlighted_lines()?
            .iter()
            .map(|line| ListItem::new(line.clone()))
            .collect();
        self.view_model.code_list = List::new(code_items)
            .block(Block::default().borders(Borders::LEFT))
            .highlight_style(highlight_style())
            .scroll_padding(self.state.config.scrolloff);

        match self.state().list_state.selected() {
            None => self.state().list_state.select(Some(len - 1)),
            Some(idx) => {
                if idx >= len {
                    self.state().list_state.select(Some(len - 1));
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        self.view_model.rect = rect;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.view_model.max_blame_len as u16),
                Constraint::Min(0),
            ])
            .split(rect);

        StatefulWidget::render(
            &self.view_model.blame_list,
            chunks[0],
            frame.buffer_mut(),
            &mut self.state.list_state,
        );

        StatefulWidget::render(
            &self.view_model.code_list,
            chunks[1],
            frame.buffer_mut(),
            &mut self.state.list_state,
        );

        self.highlight_search(
            frame,
            Rect {
                x: rect.x + chunks[1].x + 1,
                y: rect.y,
                width: chunks[1].width,
                height: chunks[1].height,
            },
        );

        if let Ok(file) = self.get_current_file() {
            self.notif(
                NotifChannel::Line,
                Some(format!(
                    "{} - line {} of {}",
                    file,
                    self.idx().unwrap_or(0) + 1,
                    self.blames.len(),
                )),
            );
        }
    }

    fn get_mapping_fields(&self) -> Vec<MappingScope> {
        vec![MappingScope::Blame]
    }

    fn get_file_rev_line(&self) -> Result<FileRevLine, Error> {
        let idx = self.idx()?;
        let commit_ref = self.blames.get(idx).ok_or_else(|| Error::StateIndex)?;

        let rev = match commit_ref {
            Some(commit) => {
                if commit.hash.starts_with('^') {
                    Some(commit.hash[1..].to_string())
                } else {
                    Some(commit.hash.clone())
                }
            }
            None => None,
        };
        let file = self.get_current_file()?;
        Ok((Some(file.clone()), rev, Some(idx + 1)))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        match action {
            Action::NextCommitBlame => {
                if self.revisions.len() == 1 {
                    return Ok(());
                }
                self.revisions.pop();
                self.files.pop();
                self.reload()?;
            }
            Action::PreviousCommitBlame => {
                let idx = self.idx()?;
                let commit_ref = self.blames.get(idx).ok_or_else(|| Error::StateIndex)?;
                let file = self.get_current_file()?;
                let (rev, prev_file) = if let Some(commit) = commit_ref {
                    if let Some('^') = commit.hash.chars().next() {
                        return Ok(());
                    }
                    let rev = format!("{}^", commit.hash);
                    let prev_file = get_previous_filename(&commit.hash, &file)?;
                    (rev, prev_file.to_string())
                } else {
                    ("HEAD".to_string(), file.clone())
                };
                self.revisions.push(Some(rev.clone()));
                self.files.push(prev_file.clone());
                self.reload()?;
            }
            _ => {
                self.run_action_generic(action, self.view_model.rect.height as usize, terminal)?;
                return Ok(());
            }
        };
        Ok(())
    }

    fn on_click(&mut self) {
        if self.view_model.rect.contains(self.state.mouse_position) {
            let delta = (self.state.mouse_position.y - self.view_model.rect.y) as usize;
            self.state
                .list_state
                .select(Some(self.state.list_state.offset() + delta));
        }
    }

    fn on_scroll(&mut self, down: bool) {
        self.on_scroll_generic(down, self.view_model.rect.height as usize, self.code.len());
    }
}
