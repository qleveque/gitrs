use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
    str::FromStr,
};

use crate::{
    action::{Action, CommandType},
    errors::Error,
};

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum MappingScope {
    Global,
    Files,
    Status,
    StatusUnstaged,
    StatusStaged,
    StatusUnmerged,
    StatusUntracked,
    Log,
    Blame,
}

impl FromStr for MappingScope {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "global" => Ok(MappingScope::Global),
            "show" => Ok(MappingScope::Files),
            "status" => Ok(MappingScope::Status),
            "unstaged" => Ok(MappingScope::StatusUnstaged),
            "staged" => Ok(MappingScope::StatusStaged),
            "unmerged" => Ok(MappingScope::StatusUnmerged),
            "untracked" => Ok(MappingScope::StatusUntracked),
            "log" => Ok(MappingScope::Log),
            "blame" => Ok(MappingScope::Blame),
            _ => return Err(Error::ParseMappingScopeError(s.to_string())),
        }
    }
}

pub type KeyBindings = HashMap<MappingScope, Vec<(String, Action)>>;

pub struct Config {
    pub scroll_off: usize,
    pub git_exe: String,
    pub smart_case: bool,
    pub clipboard_tool: String,
    pub bindings: KeyBindings,
}

impl Default for Config {
    fn default() -> Self {
        let bindings: KeyBindings = [
            (
                MappingScope::Global,
                vec![
                    ("k".to_string(), Action::Up),
                    ("<up>".to_string(), Action::Up),
                    ("j".to_string(), Action::Down),
                    ("<down>".to_string(), Action::Down),
                    ("r".to_string(), Action::Reload),
                    ("gg".to_string(), Action::First),
                    ("<home>".to_string(), Action::First),
                    ("G".to_string(), Action::Last),
                    ("<end>".to_string(), Action::Last),
                    ("q".to_string(), Action::Quit),
                    ("<esc>".to_string(), Action::Quit),
                    ("<c-u>".to_string(), Action::HalfPageUp),
                    ("<pgup>".to_string(), Action::HalfPageUp),
                    ("<c-d>".to_string(), Action::HalfPageDown),
                    ("<pgdown>".to_string(), Action::HalfPageDown),
                    ("zz".to_string(), Action::ShiftLineMiddle),
                    ("zt".to_string(), Action::ShiftLineTop),
                    ("zb".to_string(), Action::ShiftLineBottom),
                    ("/".to_string(), Action::Search),
                    ("?".to_string(), Action::SearchReverse),
                    ("<c-f>".to_string(), Action::Search),
                    (":".to_string(), Action::TypeCommand),
                    ("n".to_string(), Action::NextSearchResult),
                    ("N".to_string(), Action::PreviousSearchResult),
                    (
                        "yc".to_string(),
                        Action::Command(
                            CommandType::Async,
                            "echo %(rev) | %(clip)".to_string(),
                        ),
                    ),
                    (
                        "yf".to_string(),
                        Action::Command(
                            CommandType::Async,
                            "echo %(file) | %(clip)".to_string(),
                        ),
                    ),
                    (
                        "yy".to_string(),
                        Action::Command(
                            CommandType::Async,
                            "echo %(text) | %(clip)".to_string(),
                        ),
                    ),
                ],
            ),
            (
                MappingScope::Files,
                vec![(
                    "<cr>".to_string(),
                    Action::Command(
                        CommandType::Sync,
                        "%(git) difftool %(rev)^..%(rev) -- %(file)".to_string(),
                    ),
                )],
            ),
            (
                MappingScope::Blame,
                vec![
                    ("l".to_string(), Action::NextCommitBlame),
                    ("<right>".to_string(), Action::NextCommitBlame),
                    ("h".to_string(), Action::PreviousCommitBlame),
                    ("<left>".to_string(), Action::PreviousCommitBlame),
                    ("<cr>".to_string(), Action::OpenFilesApp),
                ],
            ),
            (
                MappingScope::Log,
                vec![
                    ("<cr>".to_string(), Action::OpenFilesApp),
                    ("c".to_string(), Action::NextCommit),
                    ("C".to_string(), Action::PreviousCommit),
                    (
                        "d".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) difftool %(rev)^..%(rev) -- %(file)".to_string(),
                        ),
                    ),
                ],
            ),
            (
                MappingScope::Status,
                vec![
                    ("t".to_string(), Action::StageUnstageFile),
                    ("<space>".to_string(), Action::StageUnstageFile),
                    ("T".to_string(), Action::StageUnstageFiles),
                    ("<tab>".to_string(), Action::SwitchView),
                    ("K".to_string(), Action::FocusUnstagedView),
                    ("J".to_string(), Action::FocusStagedView),
                    (
                        "!c".to_string(),
                        Action::Command(CommandType::Sync, "%(git) commit".to_string()),
                    ),
                    (
                        "!a".to_string(),
                        Action::Command(CommandType::Sync, "%(git) commit --amend".to_string()),
                    ),
                    (
                        "!n".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) commit --amend --no-edit".to_string(),
                        ),
                    ),
                    (
                        "!p".to_string(),
                        Action::Command(CommandType::SyncQuit, "%(git) push".to_string()),
                    ),
                    (
                        "!P".to_string(),
                        Action::Command(CommandType::SyncQuit, "%(git) push --force".to_string()),
                    ),
                ],
            ),
            (
                MappingScope::StatusUnstaged,
                vec![
                    (
                        "<cr>".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) difftool -- %(file)".to_string(),
                        ),
                    ),
                    (
                        "!r".to_string(),
                        Action::Command(CommandType::Sync, "%(git) restore %(file)".to_string()),
                    ),
                ],
            ),
            (
                MappingScope::StatusStaged,
                vec![(
                    "<cr>".to_string(),
                    Action::Command(
                        CommandType::Sync,
                        "%(git) difftool --staged -- %(file)".to_string(),
                    ),
                )],
            ),
            (
                MappingScope::StatusUntracked,
                vec![(
                    "!r".to_string(),
                    Action::Command(CommandType::Sync, "rm %(file)".to_string()),
                )],
            ),
        ]
        .into_iter()
        .collect();

        let clipboard_tool = if cfg!(windows) {
            "clip.exe"
        } else {
            "xsel"
        }.to_string();

        Config {
            scroll_off: 2,
            git_exe: "git".to_string(),
            smart_case: true,
            clipboard_tool,
            bindings,
        }
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
            let line = line?;
            let keyword = line
                .split_once(' ')
                .map(|(first, _)| first.to_string())
                .unwrap_or(line.to_string());

            match keyword.as_str() {
                "map" => {
                    let parts: Vec<&str> = line.splitn(4, ' ').collect();
                    if parts.len() < 4 {
                        continue;
                    }
                    let mode = parts[1].to_string().parse()?;
                    let key = parts[2].to_string();
                    let action_str = parts[3].to_string();

                    let action = action_str.parse::<Action>()?;
                    let bindings = config.bindings.entry(mode).or_insert_with(Vec::new);
                    // remove keybindings with the same binding
                    bindings.retain(|(k, _)| *k != key);
                    if action != Action::None {
                        bindings.push((key, action));
                    }
                }
                "set" => {
                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() < 3 {
                        continue;
                    }
                    let key = parts[1].to_string();
                    let value = parts[2].to_string();
                    match key.as_str() {
                        "scrolloff" => {
                            let number: Result<usize, _> = value.parse();
                            if let Ok(so) = number {
                                config.scroll_off = so
                            }
                        }
                        "git" => config.git_exe = value,
                        "smartcase" => config.smart_case = value == "true",
                        "clipboard" => config.clipboard_tool = value,
                        variable => return Err(Error::ParseVariableError(variable.to_string())),
                    }
                }
                _ => (),
            }
        }
    }

    Ok(config)
}
