use crate::app::{FileRevLine, GitApp};

use crate::model::{
    action::Action,
    app_state::AppState,
    config::MappingScope,
    errors::Error,
    git::{git_parse_commit, git_show_output, set_git_dir, Commit, FileStatus},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    style::{Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, StatefulWidget, Widget},
    Frame, Terminal,
};

use std::env;

struct ShowAppViewModel {
    file_list: List<'static>,
    commit_paragraph: Paragraph<'static>,
    files_rect: Rect,
}

pub struct ShowApp {
    state: AppState,
    commit: Commit,
    original_dir: std::path::PathBuf,
    view_model: ShowAppViewModel,
}

impl ShowApp {
    pub fn new(revision: Option<String>) -> Result<Self, Error> {
        let mut state = AppState::new()?;
        let original_dir = env::current_dir()?;
        set_git_dir(&state.config)?;

        let output = git_show_output(&revision, &state.config)?;
        let mut commit = git_parse_commit(&output)?;
        commit
            .files
            .sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        state.list_state.select_first();

        let mut r = Self {
            state,
            commit,
            original_dir,
            view_model: ShowAppViewModel {
                file_list: List::default(),
                commit_paragraph: Paragraph::default(),
                files_rect: Rect::default(),
            },
        };
        r.reload()?;
        Ok(r)
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
    fn state(&mut self) -> &mut AppState {
        &mut self.state
    }

    fn get_state(&self) -> &AppState {
        &self.state
    }

    fn reload(&mut self) -> Result<(), Error> {
        let file_items: Vec<ListItem> = self
            .commit
            .files
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

        self.view_model.file_list = List::new(file_items)
            .block(Block::default().borders(Borders::NONE))
            .style(Style::from(Color::White))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .scroll_padding(self.state.config.scrolloff);

        let metadata = Self::display_commit_metadata(self.commit.metadata.clone());
        self.view_model.commit_paragraph = metadata.block(Block::default().borders(Borders::NONE));
        Ok(())
    }

    fn get_text_line(&self, idx: usize) -> Option<String> {
        self.commit.files.get(idx).map(|tuple| tuple.1.clone())
    }

    fn on_exit(&mut self) -> Result<(), Error> {
        env::set_current_dir(self.original_dir.clone())
            .map_err(|_| Error::Global("could not restore initial working directory".to_string()))
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        let paragraph_len = self.commit.metadata.lines().count() + 1;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(paragraph_len as u16), Constraint::Min(5)])
            .split(rect);

        Widget::render(
            &self.view_model.commit_paragraph,
            chunks[0],
            frame.buffer_mut(),
        );
        StatefulWidget::render(
            &self.view_model.file_list,
            chunks[1],
            frame.buffer_mut(),
            &mut self.state.list_state,
        );
        self.view_model.files_rect = chunks[1];

        self.highlight_search(
            frame,
            Rect {
                x: rect.x + chunks[1].x + 2,
                y: chunks[1].y,
                width: chunks[1].width - 1,
                height: chunks[1].height,
            },
        );
    }

    fn get_mapping_fields(&self) -> Vec<MappingScope> {
        let file = self
            .commit
            .files
            .get(self.idx().unwrap_or(usize::MAX))
            .map(|(a, _)| a);
        vec![MappingScope::Show(file.copied()), MappingScope::Show(None)]
    }

    fn get_file_rev_line(&self) -> Result<FileRevLine, Error> {
        let idx = self.idx()?;
        let file = self
            .commit
            .files
            .get(idx)
            .ok_or_else(|| Error::StateIndex)?;
        let rev = Some(self.commit.hash.clone());
        Ok((Some(file.1.clone()), rev, None))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        self.run_action_generic(action, self.view_model.files_rect.height as usize, terminal)?;
        Ok(())
    }

    fn on_click(&mut self) {
        if self
            .view_model
            .files_rect
            .contains(self.state.mouse_position)
        {
            let delta = (self.state.mouse_position.y - self.view_model.files_rect.y) as usize;
            self.state
                .list_state
                .select(Some(self.state.list_state.offset() + delta));
        }
    }

    fn on_scroll(&mut self, down: bool) {
        self.on_scroll_generic(
            down,
            self.view_model.files_rect.height as usize,
            self.commit.files.len(),
        );
    }
}
