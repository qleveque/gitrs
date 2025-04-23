use crate::action::Action;
use crate::app::GitApp;
use crate::app_state::AppState;

use crate::errors::Error;
use crate::git::{git_parse_commit, git_show_output, set_git_dir, Commit, FileStatus};

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{ListState, Paragraph, StatefulWidget, Widget};

use ratatui::Frame;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use std::env;

pub struct ShowApp {
    app_state: AppState,
    commit: Commit,
    files: Vec<(FileStatus, String)>,
    file_list: List<'static>,
    commit_paragraph: Paragraph<'static>,
    state: ListState,
    files_height: usize,
    original_dir: std::path::PathBuf,
}

impl ShowApp {
    pub fn new(revision: Option<String>) -> Result<Self, Error> {
        let app_state = AppState::new()?;
        set_git_dir(&app_state.config); // TODO: is it necessary

        let output = git_show_output(&revision, &app_state.config);
        let mut lines = output.lines().map(String::from);
        let (commit, _) = git_parse_commit(&mut lines);

        let mut files = commit.files.clone();
        files.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        let file_items: Vec<ListItem> = files
            .iter()
            .map(|(status, name)| {
                let label = format!("{} {}", status.character(), name);
                let color = match status {
                    FileStatus::New => Color::Green,
                    FileStatus::Deleted => Color::Red,
                    FileStatus::Modified => Color::LightBlue,
                    _ => Color::default(),
                };
                ListItem::new(label).style(Style::from(color))
            })
            .collect();

        let file_list = List::new(file_items)
            .block(Block::default().borders(Borders::NONE))
            .style(Style::from(Color::White))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .scroll_padding(app_state.config.scroll_off);

        let metadata = Self::display_commit_metadata(commit.metadata.clone());
        let commit_paragraph = metadata.block(Block::default().borders(Borders::NONE));

        let mut state = ListState::default();
        state.select_first();

        let r = Self {
            app_state,
            commit,
            files,
            file_list,
            commit_paragraph,
            state,
            files_height: 0,
            original_dir: env::current_dir()?,
        };
        return Ok(r);
    }

    fn display_commit_metadata<'b>(metadata: String) -> Paragraph<'b> {
        let mut lines = metadata.lines();

        let mut styled_lines: Vec<Line<'static>> = Vec::new();

        if let Some(line) = lines.next() {
            styled_lines.push(Line::styled(line.to_string(), Style::from(Color::Blue)));
        }
        if let Some(line) = lines.next() {
            styled_lines.push(Line::styled(line.to_string(), Style::from(Color::Green)));
        }
        if let Some(line) = lines.next() {
            styled_lines.push(Line::styled(line.to_string(), Style::from(Color::Yellow)));
        }
        for line in lines {
            styled_lines.push(Line::styled(
                line.to_string(),
                Style::from(Color::default()),
            ));
        }

        Paragraph::new(Text::from(styled_lines))
    }
}

impl GitApp for ShowApp {
    fn get_state(&mut self) -> &mut AppState {
        &mut self.app_state
    }

    fn search_result(&mut self, state: &mut ListState, mut reversed: bool) -> Result<(), Error> {
        reversed ^= self.get_state().search_reverse;

        let mut idx = state.selected().ok_or_else(|| Error::StateIndexError)?;
        let search_string = self.get_state().search_string.clone();
        loop {
            match reversed {
                true => {
                    if idx == 0 {
                        return Err(Error::ReachedLastMachted);
                    }
                    idx -= 1;
                }
                false => idx += 1,
            }
            let tuple = self.files.get(idx as usize).ok_or_else(|| Error::ReachedLastMachted)?;
            let filename: String = tuple.1.clone();
            if filename.contains(&search_string) {
                state.select(Some(idx as usize));
                return Ok(());
            }
        }
    }

    fn on_exit(&mut self) -> Result<(), Error> {
        env::set_current_dir(self.original_dir.clone()).map_err(|_| {
            Error::GlobalError("could not restore initial working directory".to_string())
        })
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        let paragraph_len = self.commit.metadata.lines().count() + 1;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(paragraph_len as u16), Constraint::Min(5)])
            .split(rect);

        Widget::render(&self.commit_paragraph, chunks[0], frame.buffer_mut());
        StatefulWidget::render(
            &self.file_list,
            chunks[1],
            frame.buffer_mut(),
            &mut self.state,
        );
        self.files_height = chunks[1].height as usize;
    }

    fn get_mapping_fields(&mut self) -> Vec<(&str, bool)> {
        vec![("show", true)]
    }

    fn get_file_and_rev(&self) -> Result<(Option<String>, Option<String>), Error> {
        let idx = self.state.selected().ok_or_else(|| Error::StateIndexError)?;
        let file = self.files.get(idx).ok_or_else(|| Error::StateIndexError)?;
        let rev = Some(self.commit.hash.clone());
        Ok((Some(file.1.clone()), rev))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        let mut new_state = self.state.clone();
        self.run_generic_action(action, self.files_height, terminal, &mut new_state)?;
        self.state = new_state;
        return Ok(());
    }
}
