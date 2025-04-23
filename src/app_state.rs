use crate::{
    config::{parse_gitrs_config, Config}, errors::Error
};

#[derive(Clone)]
pub enum NotifType {
    Info,
    Error,
}

#[derive(Clone)]
pub struct Notif {
    pub notif_type: NotifType,
    pub message: String,
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
    pub notif: Vec<Notif>,
    pub key_combination: String,
    pub search_string: String,
    pub search_reverse: bool,
    pub command_string: String,
    pub input_state: InputState,
}

impl AppState {
    pub fn new() -> Result<Self, Error> {
        let r = Self {
            quit: false,
            config: parse_gitrs_config()?,
            notif: Vec::new(),
            key_combination: "".to_string(),
            search_string: "".to_string(),
            search_reverse: false,
            command_string: "".to_string(),
            input_state: InputState::App,
        };
        return Ok(r);
    }
}
