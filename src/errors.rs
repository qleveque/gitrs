use std::string::FromUtf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown action `{0}`")]
    ParseActionError(String),
    #[error("unknown mapping scope `{0}`")]
    ParseMappingScopeError(String),
    #[error("error parsing utf-8")]
    ParseUtf8Error(#[from] FromUtf8Error),
    #[error("unable to set variable `{0}`")]
    ParseVariableError(String),
    #[error("unable to parse button `{0}`")]
    ParseButtonError(String),
    #[error("invalid state index")]
    StateIndexError,
    #[error("reached last match")]
    ReachedLastMachted,
    #[error("i/o error")]
    IOError(#[from] std::io::Error),
    #[error("unknown filename `{0}`")]
    UnknownFilename(String),
    #[error("{0}")]
    GlobalError(String),
    #[error("could not properly parse git output")]
    GitParsingError,
    #[error("could not properly highlight code")]
    SyntaxError(#[from] syntect::Error),
}
