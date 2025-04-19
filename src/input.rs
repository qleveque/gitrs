use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};

pub struct InputManager {
    pub key_combination: String,
    pub reset_key_combination: bool,
}

impl InputManager {
    pub fn new() -> Self {
        InputManager {
            key_combination: "".to_string(),
            reset_key_combination: true,
        }
    }
    pub fn key_pressed(&mut self) -> Result<bool, std::io::Error> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let event::Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    match self.reset_key_combination {
                        true => self.key_combination = "".to_string(),
                        false => self.reset_key_combination = true,
                    };
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
                    self.key_combination = format!("{}{}", self.key_combination, key_str);
                    return Ok(true);
                }
            }
        }
        return Ok(false);
    }
}
