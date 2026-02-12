use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Get(String),
    Set(String, String),
    Del(String),
}
