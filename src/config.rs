use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
    str::FromStr,
};

use regex::Regex;

use crate::{
    action::Action,
    errors::Error,
    git::{FileStatus, StagedStatus},
};

const DEFAULT_CONFIG: &str = include_str!("../config/.gitrsrc");

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum MappingScope {
    Global,
    Files(Option<FileStatus>),
    Status(Option<StagedStatus>, Option<FileStatus>),
    Pager,
    Log,
    Show,
    Reflog,
    Diff,
    Stash,
    Blame,
}

impl FromStr for MappingScope {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');
        let key = split.next().unwrap_or("");

        match key {
            "global" => Ok(MappingScope::Global),
            "pager" => Ok(MappingScope::Pager),
            "log" => Ok(MappingScope::Log),
            "show" => Ok(MappingScope::Show),
            "reflog" => Ok(MappingScope::Reflog),
            "stash" => Ok(MappingScope::Stash),
            "blame" => Ok(MappingScope::Blame),
            "diff" => Ok(MappingScope::Diff),
            "files" => {
                let file_status = match split.next() {
                    Some(file_status_str) => Some(file_status_str.parse()?),
                    None => None,
                };
                Ok(MappingScope::Files(file_status))
            }
            "status" => {
                let staged_status = match split.next() {
                    Some(staged_status_str) => Some(staged_status_str.parse()?),
                    None => None,
                };
                let file_status = match split.next() {
                    Some(file_status_str) => Some(file_status_str.parse()?),
                    None => None,
                };
                Ok(MappingScope::Status(staged_status, file_status))
            }
            _ => Err(Error::ParseMappingScopeError(s.to_string())),
        }
    }
}

pub type KeyBindings = HashMap<MappingScope, HashMap<String, Action>>;
pub type Button = (String, Action);
pub type Buttons = HashMap<MappingScope, Vec<Button>>;

pub struct Config {
    pub scrolloff: usize,
    pub git_exe: String,
    pub smart_case: bool,
    pub scroll_step: usize,
    pub menu_bar: bool,
    pub clipboard_tool: String,
    pub use_default_mappings: bool,
    pub use_default_buttons: bool,
    pub user_bindings: KeyBindings,
    pub default_bindings: KeyBindings,
    pub user_buttons: Buttons,
    pub default_buttons: Buttons,
}

impl Config {
    fn parse_line(&mut self, line: &str, default: bool) -> Result<(), Error> {
        let mut split = line.splitn(2, ' ');
        let keyword = split.next().unwrap_or("");
        let params = split.next().unwrap_or("");

        match keyword {
            "map" => self.parse_map_line(params, default)?,
            "set" => self.parse_set_line(params)?,
            "button" => self.parse_button_line(params, default)?,
            _ => (),
        };
        return Ok(());
    }

    pub fn parse_map_line(&mut self, params: &str, default: bool) -> Result<(), Error> {
        let parts: Vec<&str> = params.splitn(3, ' ').collect();
        if parts.len() < 3 {
            return Ok(());
        }
        let mode = parts[0].to_string().parse()?;
        let key = parts[1].to_string();
        let action_str = parts[2].to_string();

        let action = action_str.parse::<Action>()?;
        let bindings = match default {
            true => &mut self.default_bindings,
            false => &mut self.user_bindings,
        };
        let mode_bindings = bindings.entry(mode).or_insert_with(HashMap::new);
        mode_bindings.insert(key, action);
        Ok(())
    }

    pub fn parse_set_line(&mut self, params: &str) -> Result<(), Error> {
        let parts: Vec<&str> = params.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return Err(Error::ParseVariableError(params.to_string()));
        }
        let key = parts[0].to_string();
        let value = parts[1].to_string();
        match key.as_str() {
            "scrolloff" => {
                let number: Result<usize, _> = value.parse();
                if let Ok(so) = number {
                    self.scrolloff = so;
                }
            }
            "git" => self.git_exe = value,
            "smart_case" => self.smart_case = value == "true",
            "scroll_step" => {
                let number: Result<usize, _> = value.parse();
                if let Ok(ss) = number {
                    self.scroll_step = ss;
                }
            }
            "menu_bar" => self.menu_bar = value == "true",
            "clipboard" => self.clipboard_tool = value,
            "default_mappings" => self.use_default_mappings = value == "true",
            "default_buttons" => self.use_default_buttons = value == "true",
            _ => return Err(Error::ParseVariableError(params.to_string())),
        }
        Ok(())
    }

    pub fn parse_button_line(&mut self, params: &str, default: bool) -> Result<(), Error> {
        let re = Regex::new(r#"^(\S+)\s+("(?:[^"]+)"|\S+)\s+(.*)"#).unwrap();
        if let Some(caps) = re.captures(&params) {
            let mode = caps[1].to_string().parse()?;
            let mut name = caps[2].to_string();
            if name.starts_with('"') && name.ends_with('"') {
                name = name[1..name.len() - 1].to_string(); // Remove quotes
            }
            let action_str = caps[3].to_string();
            let action = action_str.parse::<Action>()?;

            let buttons = match default {
                true => &mut self.default_buttons,
                false => &mut self.user_buttons,
            };
            let mode_buttons = buttons.entry(mode).or_insert_with(Vec::new);
            mode_buttons.retain(|(k, _)| *k != name);
            mode_buttons.push((name, action));
            return Ok(());
        } else {
            return Err(Error::ParseButtonError(params.to_string()));
        }
    }

    pub fn get_bindings(&self, mapping_scope: MappingScope) -> Vec<(String, Action)> {
        let user_bindings = self.user_bindings.get(&mapping_scope);
        let default_bindings = self.default_bindings.get(&mapping_scope);
        let mut merged: HashMap<String, Action> = HashMap::new();

        if let Some(default_bindings) = default_bindings {
            for (k, v) in default_bindings {
                merged.insert(k.clone(), v.clone());
            }
        }
        if let Some(user_bindings) = user_bindings {
            for (k, v) in user_bindings {
                merged.insert(k.clone(), v.clone());
            }
        }

        merged.into_iter().collect()
    }

    pub fn get_buttons(&self, mapping_scope: MappingScope) -> Vec<(String, Action)> {
        self.user_buttons
            .get(&mapping_scope)
            .into_iter()
            .chain(
                (self.use_default_buttons)
                    .then(|| self.default_buttons.get(&mapping_scope))
                    .flatten(),
            )
            .flat_map(|v| v.clone())
            .collect()
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Config {
            scrolloff: 5,
            git_exe: "git".to_string(),
            smart_case: true,
            scroll_step: 2,
            menu_bar: true,
            clipboard_tool: if cfg!(windows) { "clip.exe" } else { "xsel" }.to_string(),
            use_default_mappings: true,
            use_default_buttons: true,
            default_bindings: HashMap::new(),
            user_bindings: HashMap::new(),
            default_buttons: HashMap::new(),
            user_buttons: HashMap::new(),
        };
        for line in DEFAULT_CONFIG.lines() {
            let _ = config.parse_line(line, true);
        }
        config
    }
}

pub fn parse_gitrs_config() -> Result<Config, Error> {
    let home = std::env::var("HOME").map_err(|_| {
        Error::GlobalError("could not read `HOME` environment variable".to_string())
    })?;
    let result = fs::File::open(home + "/.gitrsrc");

    let mut config: Config = Config::default();

    if let Ok(file) = result {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            config.parse_line(&line?, false)?;
        }
    }

    Ok(config)
}
