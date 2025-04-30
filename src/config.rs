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
    pub scroll_off: usize,
    pub git_exe: String,
    pub smart_case: bool,
    pub scroll_step: usize,
    pub menu_bar: bool,
    pub clipboard_tool: String,
    pub bindings: KeyBindings,
    pub buttons: Buttons,
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
                        Action::Command(CommandType::Async, "echo '%(rev)' | %(clip)".to_string()),
                    ),
                    (
                        "yf".to_string(),
                        Action::Command(CommandType::Async, "echo '%(file)' | %(clip)".to_string()),
                    ),
                    (
                        "yy".to_string(),
                        Action::Command(CommandType::Async, "echo '%(text)' | %(clip)".to_string()),
                    ),
                ],
            ),
            (
                MappingScope::Files,
                vec![
                    (
                        "<cr>".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) difftool %(rev)^..%(rev) -- %(file)".to_string(),
                        ),
                    ),
                    (

                        "<rclick>".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) difftool %(rev)^..%(rev) -- %(file)".to_string(),
                        ),
                    )
                ],
            ),
            (
                MappingScope::Blame,
                vec![
                    ("<cr>".to_string(), Action::OpenFilesApp),
                    ("<rclick>".to_string(), Action::OpenFilesApp),
                    ("s".to_string(), Action::OpenShowApp),
                    ("l".to_string(), Action::NextCommitBlame),
                    ("<right>".to_string(), Action::NextCommitBlame),
                    ("h".to_string(), Action::PreviousCommitBlame),
                    ("<left>".to_string(), Action::PreviousCommitBlame),
                ],
            ),
            (
                MappingScope::Stash,
                vec![
                    ("<cr>".to_string(), Action::OpenFilesApp),
                    ("<rclick>".to_string(), Action::OpenFilesApp),
                    ("s".to_string(), Action::OpenShowApp),
                    (
                        "!a".to_string(),
                        Action::Command(CommandType::Sync, "%(git) stash apply".to_string()),
                    ),
                    (
                        "!p".to_string(),
                        Action::Command(CommandType::Sync, "%(git) stash pop".to_string()),
                    ),
                    (
                        "!d".to_string(),
                        Action::Command(CommandType::Sync, "%(git) stash drop".to_string()),
                    ),
                ],
            ),
            (
                MappingScope::Pager,
                vec![
                    ("<cr>".to_string(), Action::OpenFilesApp),
                    ("<rclick>".to_string(), Action::OpenFilesApp),
                    ("s".to_string(), Action::OpenShowApp),
                    ("c".to_string(), Action::PagerNextCommit),
                    ("C".to_string(), Action::PreviousCommit),
                    (
                        "d".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) difftool %(rev)^..%(rev) -- %(file)".to_string(),
                        ),
                    ),
                    (
                        "!r".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) rebase -i %(rev)^".to_string(),
                        ),
                    ),
                ],
            ),
            (
                MappingScope::Status,
                vec![
                    ("t".to_string(), Action::StageUnstageFile),
                    ("<space>".to_string(), Action::StageUnstageFile),
                    ("<rclick>".to_string(), Action::StageUnstageFile),
                    ("T".to_string(), Action::StageUnstageFiles),
                    ("<tab>".to_string(), Action::StatusSwitchView),
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
                        Action::Command(CommandType::Sync, "%(git) push".to_string()),
                    ),
                    (
                        "!P".to_string(),
                        Action::Command(CommandType::Sync, "%(git) push --force".to_string()),
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

        let buttons: Buttons = [
            (
                MappingScope::Global,
                vec![
                    (" X ".to_string(), Action::Quit),
                ],
            ),
            (
                MappingScope::Status,
                vec![
                    (" ⟳ ".to_string(), Action::Reload),
                    (
                        "Commit".to_string(),
                        Action::Command(CommandType::Sync, "%(git) commit".to_string()),
                    ),
                    (
                        "Amend".to_string(),
                        Action::Command(CommandType::Sync, "%(git) commit --amend".to_string()),
                    ),
                    (
                        "Fixup".to_string(),
                        Action::Command(CommandType::Sync, "%(git) commit --amend --no-edit".to_string()),
                    ),
                    (
                        "Push".to_string(),
                        Action::Command(CommandType::Sync, "%(git) push".to_string()),
                    ),
                    (
                        "Push Force".to_string(),
                        Action::Command(CommandType::Sync, "%(git) push --force".to_string()),
                    ),
                ],
            ),
            (
                MappingScope::StatusUnstaged,
                vec![
                    ("Stage All".to_string(), Action::StageUnstageFiles),
                    (
                        "Diff".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) difftool -- %(file)".to_string(),
                        ),
                    ),
                    (
                        "Restore".to_string(),
                        Action::Command(CommandType::Sync, "%(git) restore %(file)".to_string()),
                    ),
                ],
            ),
            (
                MappingScope::StatusStaged,
                vec![
                    ("Unstage All".to_string(), Action::StageUnstageFiles),
                    (
                        "Diff".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) difftool --staged -- %(file)".to_string(),
                        )
                    ),
                ],
            ),
            (
                MappingScope::Pager,
                vec![
                    (" ↑ ".to_string(), Action::PreviousCommit),
                    (" ↓ ".to_string(), Action::PagerNextCommit),
                    ("Show".to_string(), Action::OpenFilesApp),
                    (
                        "Diff".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) difftool %(rev)^..%(rev) -- %(file)".to_string(),
                        ),
                    ),
                    (
                        "Rebase".to_string(),
                        Action::Command(
                            CommandType::Sync,
                            "%(git) rebase -i %(rev)^".to_string(),
                        ),
                    ),
                ],
            ),
            (
                MappingScope::Blame,
                vec![
                    (" ← ".to_string(), Action::PreviousCommitBlame),
                    (" → ".to_string(), Action::NextCommitBlame),
                    ("Show".to_string(), Action::OpenFilesApp),
                ],
            ),
            (
                MappingScope::Stash,
                vec![
                    (
                        "Apply".to_string(),
                        Action::Command(CommandType::Sync, "%(git) stash apply".to_string()),
                    ),
                    (
                        "Pop".to_string(),
                        Action::Command(CommandType::Sync, "%(git) stash pop".to_string()),
                    ),
                    (
                        "Drop".to_string(),
                        Action::Command(CommandType::Sync, "%(git) stash drop".to_string()),
                    ),
                ],
            ),
        ].into_iter().collect();

        let clipboard_tool = if cfg!(windows) { "clip.exe" } else { "xsel" }.to_string();

        Config {
            scroll_off: 5,
            git_exe: "git".to_string(),
            smart_case: true,
            scroll_step: 2,
            menu_bar: true,
            clipboard_tool,
            bindings,
            buttons,
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
                                config.scroll_off = so;
                            }
                        }
                        "git" => config.git_exe = value,
                        "smartcase" => config.smart_case = value == "true",
                        "scrollstep" => {
                            let number: Result<usize, _> = value.parse();
                            if let Ok(ss) = number {
                                config.scroll_step = ss;
                            }
                        },
                        "menubar" => config.menu_bar = value == "true",
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
