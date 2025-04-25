use std::{
    io::stdout,
    process::{Command, Stdio},
};

use crossterm::{
    event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    Frame, Terminal,
};

use crate::{
    action::{Action, CommandType},
    app_state::{AppState, InputState, Notif, NotifType},
    errors::Error,
};

pub trait GitApp {
    fn draw(&mut self, frame: &mut Frame, rect: Rect);

    fn on_exit(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn reload(&mut self) -> Result<(), Error>;
    fn get_text_line(&mut self, _idx: usize) -> Option<&str>;

    fn state(&mut self) -> &mut AppState;
    fn get_mapping_fields(&mut self) -> Vec<(&str, bool)>;
    fn get_file_and_rev(&self) -> Result<(Option<String>, Option<String>), Error>;

    fn run_action(
        &mut self,
        action: &Action,
        _terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error>;

    fn notif(&mut self, notif_type: NotifType, message: &str) {
        self.state().notif.push(Notif {
            notif_type,
            message: message.to_string(),
        });
    }
    fn info(&mut self, message: &str) {
        self.notif(NotifType::Info, message);
    }
    fn error(&mut self, message: &str) {
        self.notif(NotifType::Error, message);
    }

    fn search_result(&mut self, mut reversed: bool) -> Result<(), Error> {
        reversed ^= self.state().search_reverse;

        let mut idx = self
            .state()
            .list_state
            .selected()
            .ok_or_else(|| Error::StateIndexError)?;
        let search_string = self.state().search_string.clone();
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
            let line = self
                .get_text_line(idx)
                .ok_or_else(|| Error::ReachedLastMachted)?;
            if line.contains(&search_string) {
                self.state().list_state.select(Some(idx as usize));
                return Ok(());
            }
        }
    }

    fn display_search_bar(&mut self, chunk: &mut Rect, frame: &mut Frame) {
        let search_string = if self.state().input_state == InputState::Search {
            match self.state().search_reverse {
                false => format!("/{}│", self.state().search_string),
                true => format!("?{}│", self.state().search_string),
            }
        } else {
            format!(" {}", self.state().search_string)
        };
        let title = match self.state().search_reverse {
            false => "Search",
            true => "Search (rev)",
        };
        let paragraph = Paragraph::new(search_string)
            .block(Block::default().borders(Borders::TOP).title(title));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(2)])
            .split(*chunk);
        frame.render_widget(Clear, chunks[1]);
        Widget::render(&paragraph, chunks[1], frame.buffer_mut());
        *chunk = chunks[0];
    }

    fn display_cmd_line(&mut self, chunk: &mut Rect, frame: &mut Frame) {
        let command_string = format!(":{}│", self.state().command_string);
        let paragraph = Paragraph::new(command_string)
            .block(Block::default().borders(Borders::TOP).title("Command"));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(2)])
            .split(*chunk);
        frame.render_widget(Clear, chunks[1]);
        Widget::render(&paragraph, chunks[1], frame.buffer_mut());
        *chunk = chunks[0];
    }

    fn display_notifications(&mut self, chunk: &mut Rect, frame: &mut Frame) {
        let lines: Vec<Line> = self
            .state()
            .notif
            .iter()
            .map(|notif| {
                let line_style = match notif.notif_type {
                    NotifType::Info => Style::from(Color::Blue),
                    NotifType::Error => Style::from(Color::Red),
                };
                Line::styled(notif.message.to_string(), line_style)
            })
            .collect();
        let paragraph = Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::TOP).title("Messages"));

        let len = self.state().notif.len() as u16 + 1;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(len)])
            .split(*chunk);
        frame.render_widget(Clear, chunks[1]);
        Widget::render(&paragraph, chunks[1], frame.buffer_mut());
        *chunk = chunks[0];
    }

    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        loop {
            terminal.draw(|mut frame| {
                let mut chunk = frame.area();

                self.draw(frame, chunk);

                if self.state().input_state == InputState::Search
                    || !self.state().search_string.is_empty()
                {
                    self.display_search_bar(&mut chunk, &mut frame);
                }
                if self.state().input_state == InputState::Command {
                    self.display_cmd_line(&mut chunk, &mut frame);
                }
                if !self.state().notif.is_empty() {
                    self.display_notifications(&mut chunk, &mut frame);
                }
            })?;

            let opt_action = match self.handle_user_input() {
                Err(err) => {
                    self.error(&err.to_string());
                    None
                }
                Ok(opt_action) => opt_action,
            };

            if let Some(action) = opt_action {
                match self.run_action(&action, terminal) {
                    Err(err) => self.error(&err.to_string()),
                    Ok(()) => (),
                }
                if self.state().quit {
                    break;
                }
            }

            // display key combination if multiple letters
            let key_combination = self.state().key_combination.clone();
            if self.state().notif.is_empty() && !key_combination.is_empty() {
                self.info(&format!("Keys: {}", key_combination));
            }
        }
        self.on_exit()?;

        Ok(())
    }

    fn handle_line_edited(&mut self, key_event: KeyEvent) -> Result<Option<Action>, Error> {
        let input_state = self.state().input_state.clone();
        match key_event.code {
            KeyCode::Enter => {
                // Return :command action if any
                self.state().input_state = InputState::App;
                match input_state {
                    InputState::Command => {
                        let command_string = self.state().command_string.clone();
                        self.state().command_string.clear();
                        return Ok(Some(command_string.parse::<Action>()?));
                    }
                    InputState::Search => {
                        return Ok(Some(Action::NextSearchResult));
                    }
                    InputState::App => (),
                }
            }
            KeyCode::Esc => {
                match input_state {
                    InputState::Search => self.state().search_string.clear(),
                    InputState::Command => self.state().command_string.clear(),
                    InputState::App => (),
                }
                self.state().input_state = InputState::App;
            }
            KeyCode::Backspace => {
                match input_state {
                    InputState::Search => {
                        self.state().search_string.pop();
                    }
                    InputState::Command => {
                        self.state().command_string.pop();
                    }
                    InputState::App => (),
                };
            }
            KeyCode::Char(c) => {
                match input_state {
                    InputState::Search => self.state().search_string.push(c),
                    InputState::Command => self.state().command_string.push(c),
                    InputState::App => (),
                };
            }
            _ => {
                self.error("error: this char is not handled yet");
            }
        }
        Ok(None)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<Option<Action>, Error> {
        let mut key_str = match key_event.code {
            KeyCode::Up => "up".to_string(),
            KeyCode::Down => "down".to_string(),
            KeyCode::Right => "right".to_string(),
            KeyCode::Left => "left".to_string(),
            KeyCode::Enter => "cr".to_string(),
            KeyCode::Tab => "tab".to_string(),
            KeyCode::Home => "home".to_string(),
            KeyCode::End => "end".to_string(),
            KeyCode::Esc => "esc".to_string(),
            KeyCode::PageUp => "pgup".to_string(),
            KeyCode::PageDown => "pgdown".to_string(),
            KeyCode::Char(' ') => "space".to_string(),
            key_code => key_code.to_string(),
        };

        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            key_str = format!("<c-{}>", key_str).to_string();
        } else if key_str.len() > 1 {
            key_str = format!("<{}>", key_str).to_string();
        }
        self.state().key_combination.push_str(&key_str);

        // Compute command to run from config
        let keys = self.state().key_combination.clone();
        if keys == "" {
            return Ok(None);
        }

        let bindings = self.state().config.bindings.clone();

        let mut potential = false;
        for field in [self.get_mapping_fields().as_slice(), &[("global", true)]].concat() {
            if !field.1 {
                continue;
            }
            if let Some(mode_hotkeys) = bindings.get(field.0) {
                for (key_combination, action) in mode_hotkeys {
                    if *action == Action::None {
                        continue;
                    }
                    if *key_combination == keys {
                        self.state().key_combination.clear();
                        return Ok(Some(action.clone()));
                    }
                    if key_combination.starts_with(&keys) {
                        potential = true;
                    }
                }
            }
        }
        if !potential {
            self.state().key_combination.clear();
        }
        Ok(None)
    }

    fn run_generic_action(
        &mut self,
        action: &Action,
        height: usize,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        match action {
            Action::Reload => self.reload()?,
            Action::Up => self.state().list_state.select_previous(),
            Action::Down => self.state().list_state.select_next(),
            Action::First => self.state().list_state.select_first(),
            Action::Last => self.state().list_state.select_last(),
            Action::Quit => self.state().quit = true,
            Action::HalfPageUp => self.state().list_state.scroll_up_by(height as u16 / 3),
            Action::HalfPageDown => self.state().list_state.scroll_down_by(height as u16 / 3),
            Action::CenterVertically => {
                let mut idx = self
                    .state()
                    .list_state
                    .selected()
                    .ok_or_else(|| Error::StateIndexError)?;
                if idx > height / 2 {
                    idx = idx - height / 2;
                    *self.state().list_state.offset_mut() = idx;
                } else {
                    *self.state().list_state.offset_mut() = 0;
                };
            }
            Action::Command(command_type, command) => {
                let (file, rev) = self.get_file_and_rev()?;
                self.run_command(terminal, &command_type, command.to_string(), file, rev)?;
            }
            Action::Search => {
                self.state().search_string = "".to_string();
                self.state().search_reverse = false;
                self.state().input_state = InputState::Search;
            }
            Action::SearchReverse => {
                self.state().search_string = "".to_string();
                self.state().search_reverse = true;
                self.state().input_state = InputState::Search;
            }
            Action::TypeCommand => self.state().input_state = InputState::Command,
            Action::NextSearchResult => self.search_result(false)?,
            Action::PreviousSearchResult => self.search_result(true)?,
            Action::GoTo(line) => self.state().list_state.select(Some(*line)),
            Action::None => (),
            action => {
                return Err(Error::GlobalError(format!(
                    "cannot run `{:?}` in this context",
                    action
                )));
            }
        }
        Ok(())
    }

    fn press_key() -> Result<Option<KeyEvent>, Error> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let event::Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    return Ok(Some(key_event));
                }
            }
        }
        Ok(None)
    }

    fn handle_user_input(&mut self) -> Result<Option<Action>, Error> {
        let key_event = match Self::press_key()? {
            Some(key_event) => key_event,
            None => {
                return Ok(None);
            }
        };
        self.state().notif = Vec::new();

        let input_state = self.state().input_state.clone();
        if input_state == InputState::App {
            Ok(self.handle_key_event(key_event)?)
        } else {
            Ok(self.handle_line_edited(key_event)?)
        }
    }

    fn run_command(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        command_type: &CommandType,
        mut command: String,
        filename: Option<String>,
        revision: Option<String>,
    ) -> Result<(), Error> {
        if let Some(file) = filename {
            command = command.replace("%(file)", &file);
        }
        if let Some(rev) = revision {
            command = command.replace("%(rev)", &rev);
        }

        let mut bash_proc = Command::new("bash");
        let proc = bash_proc.args(["-c", &command]);

        match command_type {
            CommandType::Async => {
                proc.stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .expect("Failed to execute command");
            }
            _ => {
                disable_raw_mode()?;
                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                terminal.show_cursor()?;

                let mut child = proc.spawn()?;
                child.wait()?;

                enable_raw_mode()?;
                execute!(stdout(), EnterAlternateScreen)?;
                terminal.hide_cursor()?;
                terminal.clear()?;
            }
        }

        match command_type {
            CommandType::SyncQuit => self.state().quit = true,
            CommandType::Sync => self.reload()?,
            _ => (),
        }

        Ok(())
    }
}
