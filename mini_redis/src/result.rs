pub type RedisResult<T> = std::result::Result<T, RedisError>;

#[derive(Debug)]
pub enum RedisError {
    Eof,
    ServerErr(mini_runtime::result::Error),
    SerdeErr(serde_json::Error),
}

impl From<mini_runtime::result::Error> for RedisError {
    fn from(e: mini_runtime::result::Error) -> Self {
        Self::ServerErr(e)
    }
}

impl From<serde_json::Error> for RedisError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeErr(e)
    }
}
