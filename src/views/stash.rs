use crate::app::{FileRevLine, GitApp};

use crate::model::{
    action::Action,
    app_state::AppState,
    config::MappingScope,
    errors::Error,
    git::{git_stash_output, Stash},
};
use crate::ui::utils::{date_to_color, highlight_style};

use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, Paragraph, StatefulWidget},
    Frame, Terminal,
};

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
        Ok(r)
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
                let (full_date, title) = line.split_once('\t').ok_or_else(|| Error::GitParsing)?;
                let (date, _) = full_date.split_once(' ').ok_or_else(|| Error::GitParsing)?;
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
        self.stashes
            .get(idx)
            .map(|stash| format!("{} {}", stash.date, stash.title))
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

    fn get_mapping_fields(&self) -> Vec<MappingScope> {
        vec![MappingScope::Stash]
    }

    fn get_file_rev_line(&self) -> Result<FileRevLine, Error> {
        Ok((None, Some(format!("stash@{{{}}}", self.idx()?)), None))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        self.run_action_generic(action, self.view_model.height, terminal)?;
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
        self.on_scroll_generic(
            down,
            self.view_model.rect.height as usize,
            self.stashes.len(),
        );
    }
}
