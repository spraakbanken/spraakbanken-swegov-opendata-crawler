use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    Internal(String),
    Reqwest(reqwest::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal(msg) => write!(f, "internal error: {}", msg),
            Self::Reqwest(_) => write!(f, "reqwest error"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Internal(_) => None,
            Self::Reqwest(err) => Some(err),
        }
    }
}
