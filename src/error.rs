use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    Internal(String),
    Reqwest(reqwest::Error),
    RequestReturnedError(reqwest::StatusCode),
    UnexpectedJsonFormat(String),
    StdIo(std::io::Error),
    JsonParsing(serde_json::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal(msg) => write!(f, "internal error: {}", msg),
            Self::Reqwest(_) => write!(f, "reqwest error"),
            Self::RequestReturnedError(code) => write!(f, "request returned {}", code),
            Self::JsonParsing(_) => write!(f, "json parsing error"),
            Self::StdIo(_) => write!(f, "io error"),
            Self::UnexpectedJsonFormat(msg) => write!(f, "unexpected json format: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Reqwest(err) => Some(err),
            Self::JsonParsing(err) => Some(err),
            Self::StdIo(err) => Some(err),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::StdIo(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::JsonParsing(value)
    }
}
