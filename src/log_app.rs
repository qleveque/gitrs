use std::io::{BufReader, Lines};
use std::process::ChildStdout;

use crate::action::Action;
use crate::app::GitApp;
use crate::app_state::AppState;

use crate::errors::Error;
use crate::git::git_log_output;
use crate::view_list::ViewList;

use ratatui::layout::Rect;

use ratatui::Frame;
use ratatui::{backend::CrosstermBackend, Terminal};

pub struct LogAppViewModel {
    list: ViewList,
    height: usize,
}

pub struct LogApp {
    state: AppState,
    lines: Vec<String>,
    iterator: Lines<BufReader<ChildStdout>>,
    loaded: bool,
    view_model: LogAppViewModel,
}

impl LogApp {
    pub fn new() -> Result<Self, Error> {
        let state = AppState::new()?;
        let iterator = git_log_output(&state.config)?;

        let mut r = Self {
            state,
            lines: Vec::new(),
            iterator,
            loaded: false,
            view_model: LogAppViewModel {
                list: ViewList::default(),
                height: 0,
            },
        };
        r.reload()?;
        r.state.list_state.select_first();
        Ok(r)
    }
}

impl GitApp for LogApp {
    fn state(&mut self) -> &mut AppState {
        &mut self.state
    }

    fn get_state(&self) -> &AppState {
        &self.state
    }

    fn reload(&mut self) -> Result<(), Error> {
        if !self.loaded {
            let chunk: Vec<_> = self.iterator.by_ref().take(100).collect::<Result<_, _>>()?;
            if chunk.is_empty() {
                self.loaded = true;
            }
            self.lines.extend(chunk);
        }
        self.view_model.list = ViewList::new(&self.lines, self.view_model.height, &mut self.state);

        Ok(())
    }

    fn get_text_line(&mut self, idx: usize) -> Option<&str> {
        match self.lines.get(idx) {
            Some(str) => Some(&str),
            None => None,
        }
    }

    fn draw(&mut self, frame: &mut Frame, rect: Rect) {
        let _ = self.reload();
        self.view_model.height = rect.height as usize;
        self.view_model.list.render(rect, frame.buffer_mut());
        self.highlight_search(frame, &self.lines, rect);
    }

    fn get_mapping_fields(&mut self) -> Vec<(&str, bool)> {
        vec![("log", true)]
    }

    fn get_file_and_rev(&self) -> Result<(Option<String>, Option<String>), Error> {
        Ok((None, None))
    }

    fn run_action(
        &mut self,
        action: &Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        self.run_generic_action(action, self.view_model.height, terminal)?;
        return Ok(());
    }
}
