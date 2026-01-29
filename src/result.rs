use std::{
    fmt::{Debug, Display},
    io,
    net::AddrParseError,
};

use backtrace::Backtrace;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    bt: Backtrace,
    type_: ErrorType,
}

impl Error {
    pub fn err_type(&self) -> &ErrorType {
        &self.type_
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "error {:?}:\n{:?}", self.type_, self.bt)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error {:?}", self.type_)
    }
}

#[derive(Debug)]
pub enum ErrorType {
    Eof,
    Blocked,
    IoError(io::Error),
    RuntimeError(String),
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
        ErrorType::RuntimeError(value).into()
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        ErrorType::RuntimeError(value.to_owned()).into()
    }
}

impl From<AddrParseError> for Error {
    fn from(e: AddrParseError) -> Self {
        ErrorType::RuntimeError(format!("address parse failed: {}", e)).into()
    }
}
