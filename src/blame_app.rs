use crate::action::Action;
use crate::app::GitApp;
use crate::app_state::AppState;
use crate::config::{Config, MappingScope};

use crate::errors::Error;
use crate::git::{git_blame_output, CommitRef};

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::StatefulWidget;

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
use syntect::util::LinesWithEndings;

use std::path::Path;

struct BlameAppViewModel {
    height: usize,
    blame_list: List<'static>,
    code_list: List<'static>,
    max_blame_len: usize,
}

pub struct BlameApp {
    state: AppState,
    file: String,
    blames: Vec<Option<CommitRef>>,
    code: Vec<String>,
    revisions: Vec<Option<String>>,
    view_model: BlameAppViewModel,
}

impl<'a> BlameApp {
    pub fn new(file: String, revision: Option<String>, line: usize) -> Result<Self, Error> {
        if !Path::new(&file).exists() {
            return Err(Error::GlobalError(
                format!("file '{}' does not exist", file).to_string(),
            ));
        }
        let revisions = vec![revision];

        let mut state = AppState::new()?;
        state.list_state.select(Some(line - 1));
        let mut instance = Self {
            state,
            file,
            blames: Vec::new(),
            code: Vec::new(),
            revisions,
            view_model: BlameAppViewModel {
                height: 0,
                blame_list: List::default(),
                code_list: List::default(),
                max_blame_len: 0,
            },
        };
        instance.reload()?;
        Ok(instance)
    }

    fn highlighted_lines(&self) -> Result<Vec<Line<'a>>, Error> {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = &ts.themes["base16-ocean.dark"];

        let file_text = self.code.join("\n");
        let path = Path::new(&self.file);
        let syntax = path
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| ps.find_syntax_by_extension(ext))
            .unwrap_or_else(|| {
                ps.find_syntax_by_first_line(&file_text)
                    .unwrap_or_else(|| ps.find_syntax_plain_text())
            });
        let mut h = HighlightLines::new(syntax, theme);

        let mut lines: Vec<Line> = Vec::new();

        for line in LinesWithEndings::from(&file_text) {
            let ranges: Vec<(SyntectStyle, String)> = h
                .highlight_line(&line, &ps)?
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
                        format!("{:>max_line_len$}", idx + 1),
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
    ) -> Result<(Vec<Option<CommitRef>>, Vec<String>), Error> {
        let output = git_blame_output(file, revision.clone(), config);

        let mut blame_column = Vec::new();
        let mut code_column = Vec::new();

        for line in output.lines() {
            let (blame, code) = line.split_once(')').ok_or_else(|| Error::GitParsingError)?;
            code_column.push(code.to_string());
            let blame_text = blame.to_string() + ")";
            let (hash, blame_text) = blame_text
                .split_once(" (")
                .ok_or_else(|| Error::GitParsingError)?;
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
            .ok_or_else(|| Error::GlobalError("blame app revision stack empty".to_string()))?;
        let (new_blames, new_code) =
            BlameApp::parse_git_blame(self.file.clone(), revision.clone(), &self.state.config)?;
        if new_blames.len() == 0 {
            self.revisions.pop();
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
            .block(Block::default())
            .style(Style::from(Color::White))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .scroll_padding(self.state.config.scroll_off);

        let code_items: Vec<ListItem> = self
            .highlighted_lines()?
            .iter()
            .map(|line| ListItem::new(line.clone()))
            .collect();
        self.view_model.code_list = List::new(code_items)
            .block(Block::default().borders(Borders::LEFT))
            .style(Style::from(Color::White))
            .highlight_style(Style::from(Color::Black).bg(Color::Gray))
            .scroll_padding(self.state.config.scroll_off);

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
        self.view_model.height = rect.height as usize;

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
    }

    fn get_mapping_fields(&mut self) -> Vec<(MappingScope, bool)> {
        vec![(MappingScope::Blame, true)]
    }

    fn get_file_rev_line(&self) -> Result<(Option<String>, Option<String>, Option<usize>), Error> {
        let idx = self.idx()?;
        let commit_ref = self.blames.get(idx).ok_or_else(|| Error::StateIndexError)?;

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
        Ok((Some(self.file.clone()), rev, Some(idx + 1)))
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
                self.reload()?;
            }
            Action::PreviousCommitBlame => {
                let idx = self.idx()?;
                let commit_ref = self.blames.get(idx).ok_or_else(|| Error::StateIndexError)?;
                let rev = if let Some(commit) = commit_ref {
                    if let Some('^') = commit.hash.chars().next() {
                        return Ok(());
                    }
                    format!("{}^", commit.hash)
                } else {
                    "HEAD".to_string()
                };
                self.revisions.push(Some(rev.clone()));
                self.reload()?;
            }
            _ => {
                self.run_generic_action(action, self.view_model.height, terminal)?;
                return Ok(());
            }
        };
        return Ok(());
    }
}
