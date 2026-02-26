use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BadResponse {
    reason: String,
}

impl BadResponse {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SSEContent {
    pub content: String,
    pub is_stop: bool,
}

impl SSEContent {
    pub fn resume(content: String) -> Self {
        Self {
            content,
            is_stop: false,
        }
    }

    pub fn stop() -> Self {
        Self {
            content: String::new(),
            is_stop: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArticalListReqBody {
    articles: Vec<String>,
}

impl ArticalListReqBody {
    pub fn new(articles: Vec<String>) -> Self {
        Self { articles }
    }
}
