use std::string::FromUtf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown action `{0}`")]
    ParseAction(String),
    #[error("unknown mapping scope `{0}`")]
    ParseMappingScope(String),
    #[error("error parsing utf-8")]
    ParseUtf8(#[from] FromUtf8Error),
    #[error("unable to set variable `{0}`")]
    ParseVariable(String),
    #[error("unable to parse button `{0}`")]
    ParseButton(String),
    #[error("invalid state index")]
    StateIndex,
    #[error("reached last match")]
    ReachedLastMachted,
    #[error("i/o error")]
    IO(#[from] std::io::Error),
    #[error("unknown filename `{0}`")]
    UnknownFilename(String),
    #[error("{0}")]
    Global(String),
    #[error("could not properly parse git output")]
    GitParsing,
    #[error("not inside a git repository")]
    NotInGitRepo,
    #[error("error running a git command")]
    GitCommand,
    #[error("could not properly highlight code")]
    Syntax(#[from] syntect::Error),
}
