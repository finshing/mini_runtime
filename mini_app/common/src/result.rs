use crate::{HttpStatus, http_parse_error};

pub type HttpResult<T> = std::result::Result<T, HttpError>;

// 无效URL
#[derive(Debug, Clone, Copy)]
pub enum InvalidUrl {
    Schema,
    Domain,
    Path,
    QueryString,
}

#[derive(Debug, Clone)]
pub enum InvalidBody {
    ReadUncomplete,
    ChunkBodyRead,
    WriteNotAllowed(String),
}

#[derive(Debug)]
pub enum HttpError {
    // 解析报错
    ParseError(String),
    IoError(std::io::Error),
    ServerError(mini_runtime::result::Error),
    SerdeErr(serde_json::Error),
    Timeout,
    InvalidUrl(InvalidUrl),
    InvalidHeader,
    // 不完整的消息体，比方长度和Content_Length不一致
    InvalidBody(InvalidBody),
    ResponseError(HttpStatus, String),
    Eof,
    // 连接关闭
    Closed,
}

impl HttpError {
    pub fn is_eof(&self) -> bool {
        match self {
            Self::Eof => true,
            Self::ServerError(e) => e.is_eof(),
            _ => false,
        }
    }
}

impl From<std::io::Error> for HttpError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<mini_runtime::result::Error> for HttpError {
    fn from(e: mini_runtime::result::Error) -> Self {
        HttpError::ServerError(e)
    }
}

impl From<String> for HttpError {
    fn from(s: String) -> Self {
        Self::ParseError(s)
    }
}

impl From<&str> for HttpError {
    fn from(s: &str) -> Self {
        Self::ParseError(s.to_owned())
    }
}

impl From<serde_json::Error> for HttpError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeErr(e)
    }
}

impl From<InvalidUrl> for HttpError {
    fn from(e: InvalidUrl) -> Self {
        Self::InvalidUrl(e)
    }
}

http_parse_error! {
    (std::string::FromUtf8Error, "from_utf8"),
    (core::str::Utf8Error, "utf8"),
    (core::num::ParseIntError, "int"),
    (core::num::ParseFloatError, "float"),
    (std::str::ParseBoolError, "bool"),
    (std::char::ParseCharError, "char"),
}
