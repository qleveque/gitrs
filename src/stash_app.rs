use crate::action::Action;
use crate::app::GitApp;
use crate::app_state::AppState;

use crate::config::MappingScope;
use crate::errors::Error;
use crate::git::{git_stash_output, Stash};
use crate::ui::{date_to_color, highlight_style};

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, StatefulWidget};

use ratatui::Frame;
use ratatui::{backend::CrosstermBackend, style::Color, widgets::List, Terminal};

struct StashAppViewModel {
    stash_list: List<'static>,
    height: usize,
    rect: Rect,
}

pub struct StashApp {
    state: AppState,
    stashes: Vec<Stash>,
    view_model: StashAppViewModel,
}

impl StashApp {
    pub fn new() -> Result<Self, Error> {
        let state = AppState::new()?;
        let mut r = Self {
            state,
            stashes: Vec::new(),
            view_model: StashAppViewModel {
                stash_list: List::default(),
                height: 0,
                rect: Rect::default(),
            },
        };
        r.reload()?;
        r.state.list_state.select_first();
        return Ok(r);
    }
}

impl GitApp for StashApp {
    fn state(&mut self) -> &mut AppState {
        &mut self.state
    }

    fn get_state(&self) -> &AppState {
        &self.state
    }

    fn reload(&mut self) -> Result<(), Error> {
        let output = git_stash_output(&self.state.config)?;
        self.stashes = output
            .lines()
            .map(|line| {
                let (full_date, title) = line
                    .split_once('\t')
                    .ok_or_else(|| Error::GitParsingError)?;
                let (date, _) = full_date
                    .split_once(' ')
                    .ok_or_else(|| Error::GitParsingError)?;
                let stash = Stash {
                    title: title.to_string(),
                    date: date.to_string(),
                };
                Ok(stash)
            })
            .collect::<Result<Vec<Stash>, Error>>()?;

        let list_items: Vec<Line> = self
            .stashes
            .iter()
            .map(|stash| {
                let spans = vec![
                    Span::styled(stash.date.clone(), Style::from(date_to_color(&stash.date))),
                    Span::raw(" "),
                    Span::styled(stash.title.clone(), Style::from(Color::White)),
                ];
                Line::from(spans)
            })
            .collect();
        self.view_model.stash_list = List::new(list_items)
            .highlight_style(highlight_style())
            .scroll_padding(self.state.config.scrolloff);

        Ok(())
    }

    fn get_text_line(&self, idx: usize) -> Option<String> {
        match self.stashes.get(idx) {
            Some(stash) => Some(format!("{} {}", stash.date, stash.title)),
            None => None,
        }
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        self.view_model.rect = rect;
        if self.stashes.is_empty() {
            let paragraph = Paragraph::new("Stash list empty");
            frame.render_widget(paragraph, rect);
            return;
        }
        StatefulWidget::render(
            &self.view_model.stash_list,
            rect,
            frame.buffer_mut(),
            &mut self.state.list_state,
        );
        self.view_model.height = rect.height as usize;

        self.highlight_search(frame, rect);
    }

    fn get_mapping_fields(&mut self) -> Vec<MappingScope> {
        vec![MappingScope::Stash]
    }

    fn get_file_rev_line(&self) -> Result<(Option<String>, Option<String>, Option<usize>), Error> {
        Ok((None, Some(format!("stash@{{{}}}", self.idx()?)), None))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        self.run_generic_action(action, self.view_model.height, terminal)?;
        return Ok(());
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
        self.standard_on_scroll(
            down,
            self.view_model.rect.height as usize,
            self.stashes.len(),
        );
    }
}
