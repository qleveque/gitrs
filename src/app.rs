use std::{
    cmp::min,
    collections::HashMap,
    io::stdout,
    process::{Command, Stdio},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Position, Rect},
    prelude::CrosstermBackend,
    widgets::{Clear, Paragraph},
    Frame, Terminal,
};
use regex::{Regex, RegexBuilder};

use crate::{
    model::{
        action::{Action, CommandType},
        app_state::{AppState, InputState, NotifChannel},
        config::{Button, MappingScope},
        errors::Error,
    },
    ui::utils::{
        display_edit_bar, display_menu_bar, display_notifications, search_highlight_style,
        SPINNER_FRAMES,
    },
    views::{
        pager::{PagerApp, PagerCommand},
        show::ShowApp,
    },
};

pub type FileRevLine = (Option<String>, Option<String>, Option<usize>);

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
            .ok_or_else(|| Error::StateIndex)
    }
    fn get_mapping_fields(&self) -> Vec<MappingScope>;
    fn get_file_rev_line(&self) -> Result<FileRevLine, Error>;

    fn run_action(
        &mut self,
        action: &Action,
        _terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error>;

    fn notif(&mut self, notif_channel: NotifChannel, message: Option<String>) {
        match message {
            Some(message) => self.state().notif.insert(notif_channel, message),
            None => self.state().notif.remove(&notif_channel),
        };
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
            .map_err(|_| Error::Global("invalid regex".to_string()))?;
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
                        self.notif(
                            NotifChannel::Error,
                            Some(Error::ReachedLastMachted.to_string()),
                        );
                        // stop search
                        self.state().current_search_idx = None;
                        self.notif(NotifChannel::Search, None);
                    }
                    return Ok(());
                }
                Some(line) => line,
            };

            if regex.is_match(&line) {
                self.state().list_state.select(Some(idx));
                // stop search
                self.state().current_search_idx = None;
                self.notif(NotifChannel::Search, None);
                return Ok(());
            }
            idx += 1;
        }
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
                        assert!(!reversed);
                        // if not fully loaded yet, we need to continue the search
                        let message =
                            format!("searching for `{}`...", self.get_state().search_string);
                        self.notif(NotifChannel::Search, Some(message));
                        self.state().current_search_idx = Some(idx);
                        return Ok(());
                    } else {
                        return Err(Error::ReachedLastMachted);
                    }
                }
                Some(line) => line,
            };

            if regex.is_match(&line) {
                self.state().list_state.select(Some(idx));
                return Ok(());
            }
        }
    }

    fn buttons(&self) -> Vec<Button> {
        let config = &self.get_state().config;
        if !config.menu_bar {
            return vec![];
        }
        let mut buttons: Vec<Button> = Vec::new();
        for field in [
            self.get_mapping_fields().as_slice(),
            &[MappingScope::Global],
        ]
        .concat()
        .iter()
        .rev()
        {
            buttons.extend(config.get_buttons(field.clone()));
        }
        buttons
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

    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Error> {
        let mut notif_time = 0;
        loop {
            terminal.draw(|frame| {
                let mut chunk = frame.area();
                let region_to_action = display_menu_bar(
                    &self.buttons(),
                    self.get_state().mouse_position,
                    self.get_state().mouse_down,
                    &mut chunk,
                    frame,
                );

                self.draw(frame, chunk);

                let state = self.get_state();

                let mut edit_bar_rect = Rect::default();
                if state.input_state != InputState::App {
                    let edit_string = match state.input_state {
                        InputState::Search => &state.search_string,
                        InputState::Command => &state.command_string,
                        InputState::App => "",
                    };
                    let edit_line_prefix = match state.input_state {
                        InputState::Search => match state.search_reverse {
                            false => "/",
                            true => "?",
                        },
                        InputState::Command => ":",
                        InputState::App => "",
                    };
                    edit_bar_rect = display_edit_bar(
                        edit_string,
                        edit_line_prefix,
                        state.edit_cursor,
                        &mut chunk,
                        frame,
                    );
                }

                display_notifications(
                    &state.notif,
                    SPINNER_FRAMES[notif_time],
                    self.loaded(),
                    &mut chunk,
                    frame,
                );
                notif_time = (notif_time + 1) % SPINNER_FRAMES.len();

                self.state().edit_bar_rect = edit_bar_rect;
                self.state().region_to_action = region_to_action;
            })?;

            // continue search if one is active
            if let Some(search_idx) = self.state().current_search_idx {
                self.continue_search(search_idx)?;
            }

            let opt_action = match self.handle_event() {
                Err(err) => {
                    self.notif(NotifChannel::Error, Some(err.to_string()));
                    None
                }
                Ok(opt_action) => opt_action,
            };

            if let Some(action) = opt_action {
                // stop search in case there is a new action
                self.state().current_search_idx = None;
                if let Err(err) = self.run_action(&action, terminal) {
                    self.notif(NotifChannel::Error, Some(err.to_string()))
                }
                if self.state().quit {
                    break;
                }
            }

            // display key combination if multiple letters
            let key_combination = self.state().key_combination.clone();
            if self.state().notif.is_empty() && !key_combination.is_empty() {
                let message = format!("keys: {}", key_combination);
                self.notif(NotifChannel::Keys, Some(message));
            }
        }
        self.on_exit()?;

        Ok(())
    }

    fn exit_input_line(&mut self) {
        let input_state = self.state().input_state.clone();
        match input_state {
            InputState::Search => self.state().search_string.clear(),
            InputState::Command => self.state().command_string.clear(),
            InputState::App => (),
        }
        self.state().edit_cursor = 0;
        self.state().input_state = InputState::App;
    }

    fn run_action_generic(
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
                self.run_command(terminal, command_type, command.to_string(), file, rev, line)?;
            }
            Action::Search => {
                self.state().search_string = "".to_string();
                self.state().search_reverse = false;
                self.state().edit_cursor = 0;
                self.state().input_state = InputState::Search;
            }
            Action::SearchReverse => {
                self.state().search_string = "".to_string();
                self.state().search_reverse = true;
                self.state().edit_cursor = 0;
                self.state().input_state = InputState::Search;
            }
            Action::TypeCommand => {
                self.state().edit_cursor = 0;
                self.state().command_string = "".to_string();
                self.state().input_state = InputState::Command;
            }
            Action::NextSearchResult => self.search_result(false)?,
            Action::PreviousSearchResult => self.search_result(true)?,
            Action::GoTo(line) => self.state().list_state.select(Some(*line)),
            Action::None => (),
            Action::Echo(message) => {
                self.notif(NotifChannel::Echo, Some(format!("echo: {}", message)))
            }
            Action::Map(line) => self.state().config.parse_map_line(line, false)?,
            Action::Set(line) => self.state().config.parse_set_line(line)?,
            Action::Button(line) => self.state().config.parse_button_line(line, false)?,
            Action::OpenGitShow | Action::OpenShowApp | Action::OpenLogApp => {
                let (_, rev, _) = self.get_file_rev_line()?;
                if let Some(rev) = rev {
                    terminal.clear()?;
                    match action {
                        Action::OpenShowApp => ShowApp::new(Some(rev))?.run(terminal)?,
                        Action::OpenGitShow => {
                            PagerApp::new(Some(PagerCommand::Show(vec![rev])))?.run(terminal)?
                        }
                        Action::OpenLogApp => {
                            PagerApp::new(Some(PagerCommand::Log(vec![rev])))?.run(terminal)?
                        }
                        _ => (),
                    }
                    terminal.clear()?;
                };
            }
            action => {
                return Err(Error::Global(format!(
                    "cannot run `{:?}` in this context",
                    action
                )));
            }
        }
        Ok(())
    }

    fn handle_event(&mut self) -> Result<Option<Action>, Error> {
        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
            match event {
                // Keyboard
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.state().notif = HashMap::new();
                    let input_state = self.state().input_state.clone();
                    return if input_state == InputState::App {
                        Ok(self.handle_key_event(key_event)?)
                    } else {
                        Ok(self.handle_line_edited(key_event)?)
                    };
                }
                // Mouse
                Event::Mouse(mouse_event) => {
                    self.state().mouse_position =
                        Position::new(mouse_event.column, mouse_event.row);
                    match mouse_event.kind {
                        MouseEventKind::Down(mouse_button) => {
                            return self.handle_click_event(mouse_button)
                        }
                        MouseEventKind::Up(_) => self.state().mouse_down = false,
                        MouseEventKind::ScrollUp => self.on_scroll(false),
                        MouseEventKind::ScrollDown => self.on_scroll(true),
                        _ => (),
                    };
                }
                _ => (),
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
        if keys.is_empty() {
            return Ok(None);
        }

        let mut potential = false;
        for field in [
            self.get_mapping_fields().as_slice(),
            &[MappingScope::Global],
        ]
        .concat()
        {
            for (key_combination, action) in self.state().config.get_bindings(field) {
                if action == Action::None {
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
        if !potential {
            self.state().key_combination.clear();
        }
        Ok(None)
    }

    fn handle_line_edited(&mut self, key_event: KeyEvent) -> Result<Option<Action>, Error> {
        let input_state = self.state().input_state.clone();
        let mut cursor = self.get_state().edit_cursor;

        let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
        let line = match input_state {
            InputState::Search => &mut self.state().search_string,
            InputState::Command => &mut self.state().command_string,
            InputState::App => return Ok(None),
        };
        match key_event.code {
            KeyCode::Enter => match input_state {
                InputState::Command => {
                    let ret = match line.parse::<Action>() {
                        Ok(action) => Ok(Some(action)),
                        Err(error) => Err(error),
                    };
                    self.state().input_state = InputState::App;
                    return ret;
                }
                InputState::Search => {
                    self.state().input_state = InputState::App;
                    return Ok(Some(Action::NextSearchResult));
                }
                InputState::App => (),
            },
            KeyCode::Esc => self.exit_input_line(),
            KeyCode::Left => {
                if !ctrl {
                    if cursor > 0 {
                        cursor -= 1;
                    }
                } else {
                    let chars: Vec<char> = line.chars().collect();
                    while cursor > 0 && chars[cursor - 1].is_whitespace() {
                        cursor -= 1;
                    }
                    while cursor > 0 && !chars[cursor - 1].is_whitespace() {
                        cursor -= 1;
                    }
                }
                self.state().edit_cursor = cursor;
            }
            KeyCode::Right => {
                if !ctrl {
                    if cursor < line.chars().count() {
                        cursor += 1;
                    }
                } else {
                    let chars: Vec<char> = line.chars().collect();
                    while cursor < chars.len() && !chars[cursor].is_whitespace() {
                        cursor += 1;
                    }
                    while cursor < chars.len() && chars[cursor].is_whitespace() {
                        cursor += 1;
                    }
                }
                self.state().edit_cursor = cursor;
            }
            KeyCode::Backspace => {
                if cursor > 0 {
                    let mut chars: Vec<char> = line.chars().collect();

                    if ctrl {
                        while cursor > 0 && chars[cursor - 1].is_whitespace() {
                            cursor -= 1;
                        }
                        let new_cursor = cursor;
                        while cursor > 0 && !chars[cursor - 1].is_whitespace() {
                            cursor -= 1;
                        }
                        chars.drain(cursor..new_cursor);
                    } else {
                        chars.remove(cursor - 1);
                        cursor -= 1;
                    }

                    *line = chars.iter().collect();
                    self.state().edit_cursor = cursor;
                }
            }
            KeyCode::Char(c) => {
                let mut new_line: Vec<char> = line.chars().collect();
                let before = new_line.len();
                new_line.insert(cursor, c);
                let after = new_line.len();
                *line = new_line.iter().collect();
                self.state().edit_cursor += after - before;
            }
            _ => {
                let message = "error: this char is not handled yet".to_string();
                self.notif(NotifChannel::Error, Some(message));
            }
        }
        Ok(None)
    }

    fn on_click(&mut self) {}
    fn handle_click_event(&mut self, mouse_button: MouseButton) -> Result<Option<Action>, Error> {
        // for the time being, cancel line inputs
        let input_state = self.get_state().input_state.clone();
        if input_state != InputState::App {
            let mouse_position = self.get_state().mouse_position;
            if self.get_state().edit_bar_rect.contains(mouse_position) {
                // TODO: line edit should be a proper object, this is not good
                let cursor = mouse_position.x as usize;
                let line = match input_state {
                    InputState::Search => &self.state().search_string,
                    InputState::Command => &self.state().command_string,
                    InputState::App => return Ok(None),
                };
                self.state().edit_cursor = if cursor > line.chars().count() {
                    line.chars().count()
                } else if cursor <= 1 {
                    0
                } else {
                    cursor - 1
                };
            } else {
                self.exit_input_line();
            }
            return Ok(None);
        }

        self.state().notif = HashMap::new();
        self.state().mouse_down = true;

        for (rect, action) in self.get_state().region_to_action.clone() {
            if rect.contains(self.get_state().mouse_position) {
                return Ok(Some(action));
            }
        }
        self.on_click();

        let mapping = match mouse_button {
            MouseButton::Right => "<rclick>",
            _ => return Ok(None),
        };

        for field in [
            self.get_mapping_fields().as_slice(),
            &[MappingScope::Global],
        ]
        .concat()
        {
            for (key_combination, action) in self.state().config.get_bindings(field) {
                if key_combination == mapping {
                    return Ok(Some(action.clone()));
                }
            }
        }

        Ok(None)
    }

    fn on_scroll(&mut self, down: bool);
    fn on_scroll_generic(&mut self, down: bool, height: usize, len: usize) {
        let scroll_step = self.get_state().config.scroll_step;
        let scrolloff = self.get_state().config.scrolloff;
        let mut index = self.idx().unwrap_or(0);

        let offset = self.state().list_state.offset_mut();
        match down {
            true => *offset += scroll_step,
            false => {
                if *offset > scroll_step {
                    *offset -= scroll_step
                } else {
                    *offset = 0
                }
            }
        };

        if *offset + scrolloff >= index {
            index = *offset + scrolloff;
        }
        if index >= len {
            index = len - 1;
        }
        if *offset + height > scrolloff && index >= *offset + height - scrolloff {
            index = *offset + height - scrolloff - 1;
        }
        self.state().list_state.select(Some(index));
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
        let command = format!(
            r#"{} || (echo "Command failed. Press enter to continue..."; read)"#,
            command
        );

        #[cfg(windows)]
        let command = format!(
            r#"{} || (echo Command failed. Press enter to continue... && pause)"#,
            command
        );

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
