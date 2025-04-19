use std::str::FromStr;

#[derive(Clone, PartialEq)]
pub enum CommandType {
    Async,
    Sync,
    SyncQuit,
}

#[derive(Clone, PartialEq)]
pub enum Action {
    Reload,
    Up,
    Down,
    First,
    Last,
    Quit,
    HalfPageUp,
    HalfPageDown,
    CenterVertically,
    Command(CommandType, String),
    StageUnstageFile,
    StageUnstageFiles,
    SwitchView,
    FocusUnstagedView,
    FocusStagedView,
    ShowCommit,
    NextCommitBlame,
    PreviousCommitBlame,
    None,
}

#[derive(Debug)]
pub struct ParseActionError;

impl FromStr for Action {
    type Err = ParseActionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "up" => Ok(Action::Up),
            "down" => Ok(Action::Down),
            "reload" => Ok(Action::Reload),
            "first" => Ok(Action::First),
            "last" => Ok(Action::Last),
            "quit" => Ok(Action::Quit),
            "half_page_up" => Ok(Action::HalfPageUp),
            "half_page_down" => Ok(Action::HalfPageDown),
            "center" => Ok(Action::CenterVertically),
            "stage_unstage_file" => Ok(Action::StageUnstageFile),
            "stage_unstage_files" => Ok(Action::StageUnstageFiles),
            "switch_view" => Ok(Action::SwitchView),
            "focus_unstaged_view" => Ok(Action::FocusUnstagedView),
            "focus_staged_view" => Ok(Action::FocusStagedView),
            "show_commit" => Ok(Action::ShowCommit),
            "next_commit_blame" => Ok(Action::NextCommitBlame),
            "previous_commit_blame" => Ok(Action::PreviousCommitBlame),
            "nop" => Ok(Action::None),
            cmd => {
                let command_type = match cmd.chars().next() {
                    Some('!') => CommandType::Sync,
                    Some('>') => CommandType::SyncQuit,
                    Some('@') => CommandType::Async,
                    _ => return Err(ParseActionError),
                };

                Ok(Action::Command(command_type, cmd[1..].to_string()))
            }
        }
    }
}
