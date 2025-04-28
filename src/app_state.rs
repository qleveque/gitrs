use std::collections::HashMap;

use ratatui::widgets::ListState;

use crate::{
    config::{parse_gitrs_config, Config},
    errors::Error,
};

#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum NotifChannel {
    Search,
    Line,
    Keys,
    Error,
}

#[derive(Clone, PartialEq)]
pub enum InputState {
    App,
    Search,
    Command,
}

pub struct AppState {
    pub quit: bool,
    pub config: Config,
    pub notif: HashMap<NotifChannel, String>,
    pub key_combination: String,
    pub search_string: String,
    pub search_reverse: bool,
    pub current_search_idx: Option<usize>,
    pub command_string: String,
    pub input_state: InputState,
    pub list_state: ListState,
}

impl AppState {
    pub fn new() -> Result<Self, Error> {
        let r = Self {
            quit: false,
            config: parse_gitrs_config()?,
            notif: HashMap::new(),
            key_combination: "".to_string(),
            search_string: "".to_string(),
            search_reverse: false,
            current_search_idx: None,
            command_string: "".to_string(),
            input_state: InputState::App,
            list_state: ListState::default(),
        };
        return Ok(r);
    }
}
