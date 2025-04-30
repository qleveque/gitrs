use std::{
    cmp::min,
    collections::HashMap,
    io::stdout,
    process::{Command, Stdio},
};

use crate::{
    app_state::NotifChannel,
    config::{Button, MappingScope},
    pager_app::{PagerApp, PagerCommand},
    ui::{bar_style, button_style, clicked_button_style, hovered_button_style, search_highlight_style},
};
use regex::{Regex, RegexBuilder};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    prelude::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Clear, Paragraph, Widget},
    Frame, Terminal,
};

use crate::{
    action::{Action, CommandType},
    app_state::{AppState, InputState},
    errors::Error,
    files_app::FilesApp,
};

const SPINNER_FRAMES: &[char] = &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];

pub trait GitApp {
    fn draw(&mut self, frame: &mut Frame, rect: Rect);

    fn on_exit(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn loaded(&self) -> bool {
        true
    }
    fn reload(&mut self) -> Result<(), Error>;
    fn get_text_line(&self, _idx: usize) -> Option<String>;

    fn state(&mut self) -> &mut AppState;
    fn get_state(&self) -> &AppState;

    fn idx(&self) -> Result<usize, Error> {
        self.get_state()
            .list_state
            .selected()
            .ok_or_else(|| Error::StateIndexError)
    }
    fn get_mapping_fields(&mut self) -> Vec<(MappingScope, bool)>;
    fn get_file_rev_line(&self) -> Result<(Option<String>, Option<String>, Option<usize>), Error>;

    fn run_action(
        &mut self,
        action: &Action,
        _terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error>;

    fn notif(&mut self, notif_channel: NotifChannel, message: String) {
        self.state().notif.insert(notif_channel, message);
    }

    fn clear_notif(&mut self, notif_channel: NotifChannel) {
        self.state().notif.remove(&notif_channel);
    }

    fn search_regex(&self) -> Result<Regex, Error> {
        let search_string = self.get_state().search_string.clone();
        let is_case_sensitive = match self.get_state().config.smart_case {
            true => search_string.chars().any(|c| c.is_uppercase()),
            false => true,
        };
        let regex = RegexBuilder::new(&search_string)
            .case_insensitive(!is_case_sensitive)
            .build()
            .map_err(|_| Error::GlobalError("invalid regex".to_string()))?;
        Ok(regex)
    }

    fn continue_search(&mut self, mut idx: usize) -> Result<(), Error> {
        let regex = self.search_regex()?;
        loop {
            let line = match self.get_text_line(idx) {
                None => {
                    if !self.loaded() {
                        // if not fully loaded yet, we need to continue the search
                        self.state().current_search_idx = Some(idx);
                    } else {
                        self.stop_continued_search();
                        self.notif(NotifChannel::Error, Error::ReachedLastMachted.to_string());
                    }
                    return Ok(());
                }
                Some(line) => line,
            };

            if regex.is_match(&line) {
                self.state().list_state.select(Some(idx as usize));
                self.stop_continued_search();
                return Ok(());
            }
            idx += 1;
        }
    }

    fn stop_continued_search(&mut self) {
        self.state().current_search_idx = None;
        self.clear_notif(NotifChannel::Search);
    }

    fn search_result(&mut self, mut reversed: bool) -> Result<(), Error> {
        reversed ^= self.state().search_reverse;
        let regex = self.search_regex()?;
        let mut idx = self.idx()?;

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
            let line = match self.get_text_line(idx) {
                None => {
                    if !self.loaded() {
                        assert_eq!(reversed, false);
                        // if not fully loaded yet, we need to continue the search
                        let message =
                            format!("searching for `{}`...", self.get_state().search_string);
                        self.notif(NotifChannel::Search, message);
                        self.state().current_search_idx = Some(idx);
                        return Ok(());
                    } else {
                        return Err(Error::ReachedLastMachted);
                    }
                }
                Some(line) => line,
            };

            if regex.is_match(&line) {
                self.state().list_state.select(Some(idx as usize));
                return Ok(());
            }
        }
    }

    fn display_menu_bar(&mut self, chunk: &mut Rect, frame: &mut Frame) -> Vec<(Rect, Action)> {
        if !self.state().config.menu_bar {
            return vec![];
        }
        let mut buttons: Vec<Button> = Vec::new();
        for field in [
            self.get_mapping_fields().as_slice(),
            &[(MappingScope::Global, true)],
        ]
        .concat().iter().rev()
        {
            if field.1 {
                if let Some(new_buttons) = self.state().config.buttons.get(&field.0) {
                    buttons.extend(new_buttons.clone());
                }
            }
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(*chunk);

        let mut constraints = vec![Constraint::Length(1)];
        for button in &buttons {
            constraints.push(Constraint::Length(button.0.chars().count() as u16));
            constraints.push(Constraint::Length(1));
        }

        let horizontal_chunks = Layout::default()
            .constraints(constraints)
            .direction(Direction::Horizontal)
            .split(chunks[0]);


        let paragraph = Paragraph::default().style(bar_style());
        Widget::render(&paragraph, chunks[0], frame.buffer_mut());

        let mut region_to_action = Vec::new();

        for (idx, button) in buttons.iter().enumerate() {
            let chunk = horizontal_chunks[2 * idx + 1];
            let style = if chunk.contains(self.get_state().mouse_position) {
                if self.get_state().mouse_down {
                    clicked_button_style()
                } else {
                    hovered_button_style()
                }
            } else {
                button_style()
            };
            let paragraph = Paragraph::new(button.0.to_string()).style(style);
            Widget::render(&paragraph, chunk, frame.buffer_mut());
            region_to_action.push((chunk, button.1.clone()))
        }
        *chunk = chunks[1];
        region_to_action
    }

    fn display_search_bar(&mut self, chunk: &mut Rect, frame: &mut Frame) {
        let search_string = match self.state().search_reverse {
            false => format!("/{}│", self.state().search_string),
            true => format!("?{}│", self.state().search_string),
        };
        let paragraph = Paragraph::new(search_string);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(*chunk);
        frame.render_widget(Clear, chunks[1]);
        Widget::render(&paragraph, chunks[1], frame.buffer_mut());
        *chunk = chunks[0];
    }

    fn highlight_search(&self, frame: &mut Frame, rect: Rect) {
        if self.get_state().search_string.is_empty() || rect.width == 0 {
            return;
        }
        let first = self.get_state().list_state.offset();
        let last = first + rect.height as usize;
        if let Ok(regex) = self.search_regex() {
            for idx in first..last {
                if let Some(line) = self.get_text_line(idx) {
                    for mat in regex.find_iter(&line) {
                        let match_start = mat.start() as u16;
                        let match_width = (mat.end() - mat.start()) as u16;
                        if match_start >= rect.width {
                            // result too far on the right
                            continue;
                        }
                        let x = match_start;
                        let x2 = min(x + match_width, rect.width);
                        let width = x2 - x;

                        let draw_rect = Rect {
                            x: rect.x + x,
                            y: rect.y + (idx - first) as u16,
                            width,
                            height: 1,
                        };
                        frame.render_widget(Clear, draw_rect);
                        frame.render_widget(
                            Paragraph::new(mat.as_str()).style(search_highlight_style()),
                            draw_rect,
                        );
                    }
                }
            }
        }
    }

    fn display_cmd_line(&mut self, chunk: &mut Rect, frame: &mut Frame) {
        let command_string = format!(":{}│", self.state().command_string);
        let paragraph = Paragraph::new(command_string);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(*chunk);
        frame.render_widget(Clear, chunks[1]);
        Widget::render(&paragraph, chunks[1], frame.buffer_mut());
        *chunk = chunks[0];
    }

    fn display_notifications(&mut self, chunk: &mut Rect, frame: &mut Frame, loading_char: char) {
        let loaded = self.loaded();

        let mut notif_vec: Vec<_> = self.state().notif.iter().collect();
        notif_vec.sort_by_key(|(notif_channel, _)| *notif_channel);

        let lines: Vec<Line> = notif_vec.into_iter()
            .map(|(notif_channel, message)| {
                let line_style = match notif_channel {
                    NotifChannel::Error => Style::from(Color::Red),
                    _ => Style::from(Color::Blue),
                };
                let mut message = message.clone();
                match notif_channel {
                    NotifChannel::Search => {
                        message.push(' ');
                        message.push(loading_char);
                    },
                    NotifChannel::Line if !loaded => {
                        message.push_str("... ");
                        message.push(loading_char);
                    }
                    _ => (),
                };
                Line::styled(message.to_string(), line_style)
            })
            .collect();
        let paragraph = Paragraph::new(Text::from(lines))
            .style(bar_style());

        let len = self.state().notif.len() as u16;
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
        let mut notif_time = 0;

        loop {
            terminal.draw(|mut frame| {
                let mut chunk = frame.area();
                self.state().region_to_action = self.display_menu_bar(&mut chunk, &mut frame);

                self.draw(frame, chunk);

                if self.state().input_state == InputState::Search {
                    self.display_search_bar(&mut chunk, &mut frame);
                }
                if self.state().input_state == InputState::Command {
                    self.display_cmd_line(&mut chunk, &mut frame);
                }
                if !self.state().notif.is_empty() {
                    self.display_notifications(&mut chunk, &mut frame, SPINNER_FRAMES[notif_time]);
                    notif_time = (notif_time + 1) % 8;
                }
            })?;

            // continue search if one is active
            if let Some(search_idx) = self.state().current_search_idx {
                self.continue_search(search_idx)?;
            }

            let opt_action = match self.handle_user_event() {
                Err(err) => {
                    self.notif(NotifChannel::Error, err.to_string());
                    None
                }
                Ok(opt_action) => opt_action,
            };

            if let Some(action) = opt_action {
                // stop search in case there is a new action
                self.state().current_search_idx = None;
                match self.run_action(&action, terminal) {
                    Err(err) => self.notif(NotifChannel::Error, err.to_string()),
                    Ok(()) => (),
                }
                if self.state().quit {
                    break;
                }
            }

            // display key combination if multiple letters
            let key_combination = self.state().key_combination.clone();
            if self.state().notif.is_empty() && !key_combination.is_empty() {
                let message = format!("keys: {}", key_combination);
                self.notif(NotifChannel::Keys, message);
            }
        }
        self.on_exit()?;

        Ok(())
    }


    fn cancel_input(&mut self) {
        let input_state = self.state().input_state.clone();
        match input_state {
            InputState::Search => self.state().search_string.clear(),
            InputState::Command => self.state().command_string.clear(),
            InputState::App => (),
        }
        self.state().input_state = InputState::App;
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
            KeyCode::Esc => self.cancel_input(),
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
                let message = "error: this char is not handled yet".to_string();
                self.notif(NotifChannel::Error, message);
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
        for field in [
            self.get_mapping_fields().as_slice(),
            &[(MappingScope::Global, true)],
        ]
        .concat()
        {
            if !field.1 {
                continue;
            }
            if let Some(mode_hotkeys) = bindings.get(&field.0) {
                for (key_combination, action) in mode_hotkeys {
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
            Action::HalfPageUp => self.state().list_state.scroll_up_by(height as u16 / 2),
            Action::HalfPageDown => self.state().list_state.scroll_down_by(height as u16 / 2),
            Action::ShiftLineMiddle => {
                let idx = self.idx()?;
                if idx > height / 2 {
                    *self.state().list_state.offset_mut() = idx - height / 2;
                } else {
                    *self.state().list_state.offset_mut() = 0;
                };
            }
            Action::ShiftLineTop => {
                *self.state().list_state.offset_mut() = self.idx()?;
            }
            Action::ShiftLineBottom => {
                let idx = self.idx()?;
                if idx > height {
                    *self.state().list_state.offset_mut() = idx - height;
                } else {
                    *self.state().list_state.offset_mut() = 0;
                };
            }
            Action::Command(command_type, command) => {
                let (file, rev, line) = self.get_file_rev_line()?;
                self.run_command(
                    terminal,
                    &command_type,
                    command.to_string(),
                    file,
                    rev,
                    line,
                )?;
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
            Action::OpenFilesApp | Action::OpenShowApp => {
                let (_, rev, _) = self.get_file_rev_line()?;
                if let Some(rev) = rev {
                    terminal.clear()?;
                    match action {
                        Action::OpenFilesApp => FilesApp::new(Some(rev))?.run(terminal)?,
                        Action::OpenShowApp => {
                            PagerApp::new(Some(PagerCommand::Show(vec![rev])))?.run(terminal)?
                        }
                        _ => (),
                    }
                    terminal.clear()?;
                };
            }
            action => {
                return Err(Error::GlobalError(format!(
                    "cannot run `{:?}` in this context",
                    action
                )));
            }
        }
        Ok(())
    }

    fn handle_user_event(&mut self) -> Result<Option<Action>, Error> {
        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
            match event {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.state().notif = HashMap::new();
                    let input_state = self.state().input_state.clone();
                    return if input_state == InputState::App {
                        Ok(self.handle_key_event(key_event)?)
                    } else {
                        Ok(self.handle_line_edited(key_event)?)
                    };
                },
                Event::Mouse(mouse_event) => {
                    self.state().mouse_position = Position::new(mouse_event.column, mouse_event.row);
                    match mouse_event.kind {
                        MouseEventKind::Down(mouse_button) => {
                            // for the time being, cancel line inputs
                            self.cancel_input();
                            self.state().notif = HashMap::new();
                            self.state().mouse_down = true;
                            return Ok(self.handle_click_event(mouse_button)?);
                        },
                        MouseEventKind::Up(_) => {
                            self.state().mouse_down = false;
                        },
                        MouseEventKind::ScrollUp => {
                            self.on_scroll(false);
                        },
                        MouseEventKind::ScrollDown => {
                            self.on_scroll(true);
                        },
                        _ => (),
                    };
                },
                _ => ()
            }
        }
        return Ok(None);
    }


    fn on_scroll(&mut self, down: bool);
    fn standard_on_scroll(
        &mut self,
        down: bool,
        height: usize,
        len: usize
    ) {
        let scroll_step = self.get_state().config.scroll_step;
        let scroll_off = self.get_state().config.scroll_off;
        let mut index = self.idx().unwrap_or(0);

        let offset = self.state().list_state.offset_mut();
        match down {
            true => *offset += scroll_step,
            false => if *offset > scroll_step {
                *offset -= scroll_step
            } else {
                *offset = 0
            },
        };

        if *offset + scroll_off >= index {
            index = *offset + scroll_off;
        }
        if index >= len {
            index = len - 1;
        }
        if *offset + height > scroll_off && index >= *offset + height - scroll_off {
            index = *offset + height - scroll_off - 1;
        }
        self.state().list_state.select(Some(index));
    }

    fn on_click(&mut self) {}

    fn handle_click_event(&mut self, mouse_button: MouseButton) -> Result<Option<Action>, Error> {
        for (rect, action) in self.get_state().region_to_action.clone() {
            if rect.contains(self.get_state().mouse_position) {
                return Ok(Some(action));
            }
        }
        self.on_click();

        let mapping = match mouse_button {
            MouseButton::Right => "<rclick>",
            _ => {
                return Ok(None)
            },
        };

        let bindings = self.state().config.bindings.clone();

        for field in [
            self.get_mapping_fields().as_slice(),
            &[(MappingScope::Global, true)],
        ]
        .concat()
        {
            if field.1 {
                if let Some(mode_hotkeys) = bindings.get(&field.0) {
                    for (key_combination, action) in mode_hotkeys {
                        if key_combination == mapping {
                            return Ok(Some(action.clone()));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn run_command(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        command_type: &CommandType,
        mut command: String,
        file: Option<String>,
        rev: Option<String>,
        line_number: Option<usize>,
    ) -> Result<(), Error> {
        if let Some(file) = file {
            command = command.replace("%(file)", &file);
        }
        if let Some(rev) = rev {
            command = command.replace("%(rev)", &rev);
        }
        if let Some(line_number) = line_number {
            command = command.replace("%(line)", &format!("{}", line_number));
        }
        if let Ok(idx) = self.idx() {
            if let Some(line) = self.get_text_line(idx) {
                command = command.replace("%(text)", &line);
            }
        }
        command = command.replace("%(clip)", &self.state().config.clipboard_tool);
        command = command.replace("%(git)", &self.state().config.git_exe);

        #[cfg(unix)]
        let shell = ("bash", "-c");

        #[cfg(windows)]
        let shell = ("cmd", "/C");

        #[cfg(unix)]
        let command = format!(r#"{} || (echo "Command failed. Press enter to continue..."; read)"#, command);

        #[cfg(windows)]
        let command = format!(r#"{} || (echo Command failed. Press enter to continue... && pause)"#, command);

        let mut bash_proc = Command::new(shell.0);
        let proc = bash_proc.args([shell.1, &command]);

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
                execute!(stdout(), DisableMouseCapture)?;
                terminal.show_cursor()?;

                let mut child = proc.spawn()?;
                child.wait()?;

                enable_raw_mode()?;
                execute!(stdout(), EnableMouseCapture)?;
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
