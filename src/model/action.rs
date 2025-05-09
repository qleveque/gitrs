use std::str::FromStr;

use crate::model::errors::Error;

#[derive(Clone, PartialEq, Debug)]
pub enum CommandType {
    Async,
    Sync,
    SyncQuit,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Action {
    Reload,
    Up,
    Down,
    First,
    Last,
    Quit,
    HalfPageUp,
    HalfPageDown,
    ShiftLineMiddle,
    ShiftLineTop,
    ShiftLineBottom,
    Search,
    SearchReverse,
    NextSearchResult,
    PreviousSearchResult,
    TypeCommand,
    Command(CommandType, String),
    GoTo(usize),
    StageUnstageFile,
    StageUnstageFiles,
    StatusSwitchView,
    FocusUnstagedView,
    FocusStagedView,
    OpenGitShow,
    OpenLogApp,
    OpenShowApp,
    NextCommitBlame,
    PreviousCommitBlame,
    PagerNextCommit,
    PreviousCommit,
    StashPop,
    StashApply,
    StashDrop,
    Echo(String),
    Set(String),
    Map(String),
    Button(String),
    None,
}

impl FromStr for Action {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(2, ' ');
        let key = split.next().unwrap_or("");
        let parameters = split.next().unwrap_or("");
        match key {
            "up" => Ok(Action::Up),
            "down" => Ok(Action::Down),
            "reload" => Ok(Action::Reload),
            "first" => Ok(Action::First),
            "last" => Ok(Action::Last),
            "quit" => Ok(Action::Quit),
            "half_page_up" => Ok(Action::HalfPageUp),
            "half_page_down" => Ok(Action::HalfPageDown),
            "shift_line_middle" => Ok(Action::ShiftLineMiddle),
            "shift_line_top" => Ok(Action::ShiftLineTop),
            "shift_line_bottom" => Ok(Action::ShiftLineBottom),
            "next_search_result" => Ok(Action::NextSearchResult),
            "previous_search_result" => Ok(Action::PreviousSearchResult),
            "type_command" => Ok(Action::TypeCommand),
            "search" => Ok(Action::Search),
            "search_reverse" => Ok(Action::SearchReverse),
            "stage_unstage_file" => Ok(Action::StageUnstageFile),
            "stage_unstage_files" => Ok(Action::StageUnstageFiles),
            "status_switch_view" => Ok(Action::StatusSwitchView),
            "focus_unstaged_view" => Ok(Action::FocusUnstagedView),
            "focus_staged_view" => Ok(Action::FocusStagedView),
            "open_git_show" => Ok(Action::OpenGitShow),
            "open_log_app" => Ok(Action::OpenLogApp),
            "open_show_app" => Ok(Action::OpenShowApp),
            "next_commit_blame" => Ok(Action::NextCommitBlame),
            "previous_commit_blame" => Ok(Action::PreviousCommitBlame),
            "pager_next_commit" => Ok(Action::PagerNextCommit),
            "pager_previous_commit" => Ok(Action::PreviousCommit),
            "stash_pop" => Ok(Action::StashPop),
            "stash_apply" => Ok(Action::StashApply),
            "stash_drop" => Ok(Action::StashDrop),
            "echo" => Ok(Action::Echo(parameters.to_string())),
            "set" => Ok(Action::Set(parameters.to_string())),
            "map" => Ok(Action::Map(parameters.to_string())),
            "button" => Ok(Action::Button(parameters.to_string())),
            "nop" => Ok(Action::None),
            "goto" => {
                if let Ok(number) = parameters.parse::<usize>() {
                    if number > 0 {
                        return Ok(Action::GoTo(number - 1));
                    }
                };
                Ok(Action::GoTo(0))
            }
            _ => {
                if let Ok(number) = s.parse::<usize>() {
                    if number > 0 {
                        return Ok(Action::GoTo(number - 1));
                    }
                }
                let command_type = match s.chars().next() {
                    Some('!') => CommandType::Sync,
                    Some('>') => CommandType::SyncQuit,
                    Some('@') => CommandType::Async,
                    _ => return Err(Error::ParseAction(s.to_string())),
                };

                Ok(Action::Command(command_type, s[1..].to_string()))
            }
        }
    }
}
