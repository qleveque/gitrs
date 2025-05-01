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

const DEFAULT_CONFIG: &str = r#"
# Don't get stuck
map global q quit
map global <esc> quit

# Global shortcuts that applies to all the views
map global k up
map global <up> up
map global j down
map global <down> down
map global gg first
map global <home> first
map global G last
map global <end> last
map global <c-u> half_page_u
map global <pgup> half_page_up
map global <c-d> half_page_down
map global <pgdown> half_page_down
map global zz shift_line_middle
map global zt shift_line_top
map global zb shift_line_bottom
map global / search
map global ? search_reverse
map global <c-f> search
map global : type_command
map global n next_search_result
map global N previous_search_result
map global yc !echo '%(rev)' | %(clip)".to_string()
map global yf !echo '%(file)' | %(clip)".to_string()
map global yy !echo '%(text)' | %(clip)".to_string()
map global s open_show_app
map global d !%(git) difftool %(rev)^..%(rev) -- %(file)

# Shortcuts that applies to pager views, so log, show and reflog
map pager <cr> open_files_app
map pager <rclick> open_files_app
map pager c pager_next_commit
map pager C previous_commit
map pager !r !%(git) rebase -i %(rev)^

# Files view
map files <cr> !%(git) difftool %(rev)^..%(rev) -- %(file)
map files <rclick> !%(git) difftool %(rev)^..%(rev) -- %(file)

# Blame view
map blame <cr> open_files_app
map blame <rclick> open_files_app
map blame l next_commit_blame
map blame <right> next_commit_blame
map blame h previous_commit_blame
map blame <left> previous_commit_blame

# Stash view
map stash <cr> open_files_app
map stash <rclick> open_files_app
map stash !a !%(git) stash apply
map stash !p !%(git) stash pop
map stash !d !%(git) stash drop

# Status view
map status <cr> stage_unstage_file
map status <rclick> stage_unstage_file
map status r reload
map status t stage_unstage_file
map status T stage_unstage_files
map status <tab> status_switch_view
map status K focus_unstaged_view
map status J focus_staged_view
map status !c !%(git) commit
map status !a !%(git) commit --amend
map status !n !%(git) commit --amend --no-edit
map status !p !%(git) push
map status !P !%(git) push --force
map unstaged !r %(git) restore %(file)
map untracked !r rm %(file)
map unstaged d %(git) difftool -- %(file)
map staged d %(git) difftool --staged -- %(file)

# Buttons
button global " X " quit

button pager " ↵ " open_files_app
button pager " ↑ " previous_commit
button pager " ↓ " pager_next_commit
button pager Diff !%(git) difftool %(rev)^..%(rev) -- %(file)
button pager Rebase !%(git) rebase -i %(rev)^

button files " ↵ " !%(git) difftool %(rev)^..%(rev) -- %(file)

button blame " ↵ " open_files_app
button blame " ← " previous_commit_blame
button blame " → " next_commit_blame

button stash " ↵ " open_files_app
button stash Apply !%(git) stash apply
button stash Pop !%(git) stash pop
button stash Drop !%(git) stash drop

button status " ↵ " stage_unstage_file
button status " ⟳ " reload
button unstaged Diff !%(git) difftool -- %(file)
button staged Diff !%(git) difftool --staged -- %(file)
button unstaged "Stage All" stage_unstage_files
button staged "Unstage All" stage_unstage_files
button status Commit !%(git) commit
button status Amend !%(git) commit --amend
button status Fixup !%(git) commit --amend --no-edit
button status Push !%(git) push
button status Push Force !%(git) push --force
button unstaged Restore !%(git) restore %(file)
"#;


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

impl Config {
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
                            self.scroll_off = so;
                        }
                    }
                    "git" => self.git_exe = value,
                    "smartcase" => self.smart_case = value == "true",
                    "scrollstep" => {
                        let number: Result<usize, _> = value.parse();
                        if let Ok(ss) = number {
                            self.scroll_step = ss;
                        }
                    },
                    "menubar" => self.menu_bar = value == "true",
                    "clipboard" => self.clipboard_tool = value,
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
            scroll_off: 5,
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

        for line in reader.lines() {
            config.parse_line(&line?)?;
        }
    }

    Ok(config)
}
