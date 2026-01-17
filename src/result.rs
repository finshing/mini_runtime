use std::{fmt::Debug, io};

use backtrace::Backtrace;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    bt: Backtrace,
    type_: ErrorType,
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "error {:?}:\n{:?}", self.type_, self.bt)
    }
}

#[derive(Debug)]
pub enum ErrorType {
    Eof,
    Blocked,
    IoError(io::Error),
    CommonError(String),
}

impl From<ErrorType> for Error {
    fn from(value: ErrorType) -> Self {
        Self {
            bt: Backtrace::new(),
            type_: value,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        let et = match e.kind() {
            io::ErrorKind::UnexpectedEof => ErrorType::Eof,
            io::ErrorKind::WouldBlock => ErrorType::Blocked,
            _ => ErrorType::IoError(e),
        };

        et.into()
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        ErrorType::CommonError(value).into()
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        ErrorType::CommonError(value.to_owned()).into()
    }
}
