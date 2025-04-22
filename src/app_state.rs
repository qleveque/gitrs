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

pub struct AppState {
    pub quit: bool,
    pub config: Config,
    pub notif: Option<Notif>,
    pub key_combination: String,
}

impl AppState {
    pub fn new() -> Result<Self, Error> {
        let r = Self {
            quit: false,
            config: parse_gitrs_config()?,
            notif: None,
            key_combination: "".to_string(),
        };
        return Ok(r);
    }
}
