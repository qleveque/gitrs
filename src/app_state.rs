use crate::{
    config::{parse_gitrs_config, Config}, errors::Error
};

pub struct AppState {
    pub quit: bool,
    pub config: Config,
}

impl AppState {
    pub fn new() -> Result<Self, Error> {
        let r = Self {
            quit: false,
            config: parse_gitrs_config()?,
        };
        return Ok(r);
    }
}
