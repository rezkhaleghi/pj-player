use std::fmt;
use std::io;

#[derive(Debug)]
pub enum AppError {
    Io(io::Error),

    Http(reqwest::Error),

    Utf8(std::string::FromUtf8Error),

    Process {
        command: String,
        status: Option<i32>,
        stderr: String,
    },

    Message(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {}", error),

            Self::Http(error) => write!(f, "HTTP error: {}", error),

            Self::Utf8(error) => write!(f, "UTF-8 error: {}", error),

            Self::Process {
                command,
                status,
                stderr,
            } => {
                match status {
                    Some(code) => {
                        write!(f, "{} failed with exit code {}", command, code)?;
                    }
                    None => {
                        write!(f, "{} failed", command)?;
                    }
                }

                if !stderr.trim().is_empty() {
                    write!(f, ": {}", stderr.trim())?;
                }

                Ok(())
            }

            Self::Message(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for AppError {}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<std::string::FromUtf8Error> for AppError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Self::Utf8(error)
    }
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        Self::Message(message.to_string())
    }
}
