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
};

const DEFAULT_CONFIG: &str = include_str!("../config/.gitrsrc");

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum MappingScope {
    Global,
    Files,
    Status,
    StatusUnstaged,
    StatusStaged,
    StatusUnmerged,
    StatusUntracked,
    StatusModified,
    StatusDeleted,
    Pager,
    Log,
    Show,
    Reflog,
    Stash,
    Blame,
}

impl FromStr for MappingScope {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "global" => Ok(MappingScope::Global),
            "files" => Ok(MappingScope::Files),
            "status" => Ok(MappingScope::Status),
            "unstaged" => Ok(MappingScope::StatusUnstaged),
            "staged" => Ok(MappingScope::StatusStaged),
            "unmerged" => Ok(MappingScope::StatusUnmerged),
            "modified" => Ok(MappingScope::StatusModified),
            "deleted" => Ok(MappingScope::StatusDeleted),
            "untracked" => Ok(MappingScope::StatusUntracked),
            "pager" => Ok(MappingScope::Pager),
            "log" => Ok(MappingScope::Log),
            "show" => Ok(MappingScope::Show),
            "reflog" => Ok(MappingScope::Reflog),
            "stash" => Ok(MappingScope::Stash),
            "blame" => Ok(MappingScope::Blame),
            _ => return Err(Error::ParseMappingScopeError(s.to_string())),
        }
    }
}

pub type KeyBindings = HashMap<MappingScope, Vec<(String, Action)>>;
pub type Button = (String, Action);
pub type Buttons = HashMap<MappingScope, Vec<Button>>;

pub struct Config {
    pub scrolloff: usize,
    pub git_exe: String,
    pub smart_case: bool,
    pub scroll_step: usize,
    pub menu_bar: bool,
    pub clipboard_tool: String,
    pub bindings: KeyBindings,
    pub buttons: Buttons,
}

impl Config {
    fn check_no_default_config(&mut self, line: &str) {
        if line == "set default_config false" {
            self.bindings.clear();
            self.buttons.clear();
        }
    }
    fn parse_line(&mut self, line: &str) -> Result<(), Error> {
        let keyword = line
            .split_once(' ')
            .map(|(first, _)| first.to_string())
            .unwrap_or(line.to_string());

        match keyword.as_str() {
            "map" => {
                let parts: Vec<&str> = line.splitn(4, ' ').collect();
                if parts.len() < 4 {
                    return Ok(());
                }
                let mode = parts[1].to_string().parse()?;
                let key = parts[2].to_string();
                let action_str = parts[3].to_string();

                let action = action_str.parse::<Action>()?;
                let bindings = self.bindings.entry(mode).or_insert_with(Vec::new);
                // remove keybindings with the same binding
                bindings.retain(|(k, _)| *k != key);
                if action != Action::None {
                    bindings.push((key, action));
                }
            }
            "set" => {
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() < 3 {
                    return Ok(());
                }
                let key = parts[1].to_string();
                let value = parts[2].to_string();
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
                    },
                    "menu_bar" => self.menu_bar = value == "true",
                    "clipboard" => self.clipboard_tool = value,
                    "default_config" => (),
                    variable => return Err(Error::ParseVariableError(variable.to_string())),
                }
            },
            "button" => {
                let re = Regex::new(r#"^button\s+(\S+)\s+("(?:[^"]+)"|\S+)\s+(.*)"#).unwrap();
                if let Some(caps) = re.captures(&line) {
                    let mode = caps[1].to_string().parse()?;
                    let mut name = caps[2].to_string();
                    if name.starts_with('"') && name.ends_with('"') {
                        name = name[1..name.len()-1].to_string(); // Remove quotes
                    }
                    let action_str = caps[3].to_string();
                    let action = action_str.parse::<Action>()?;
                    let buttons = self.buttons.entry(mode).or_insert_with(Vec::new);
                    buttons.retain(|(k, _)| *k != name);
                    if action != Action::None {
                        buttons.push((name, action));
                    }
                }
            }
            _ => (),
        }
        return Ok(());
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
            bindings: HashMap::new(),
            buttons: HashMap::new(),
        };
        for line in DEFAULT_CONFIG.lines() {
            let _ = config.parse_line(line);
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
        let lines: Vec<String> = reader
            .lines()
            .collect::<Result<_, _>>()?; // collect into Vec<String> or return error if any line fails

        for line in &lines {
            config.check_no_default_config(line);
        }

        for line in &lines {
            config.parse_line(line)?;
        }
    }

    Ok(config)
}
