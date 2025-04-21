use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unable to parse custom action `{0}`")]
    ParseActionError(String),
    #[error("invalid state index")]
    StateIndexError,
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
