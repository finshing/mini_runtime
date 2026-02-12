use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Exist(String),
    NotFound,
    Err(String),
    Ok,
}
