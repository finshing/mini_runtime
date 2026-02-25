use std::fs::read_dir;

use common::result::HttpResult;
use serde::Serialize;

use crate::{request::ServerRequest, response::ServerResponse, route::THttpMethodHandler};

#[derive(Serialize)]
struct ArticalList {
    articles: Vec<String>,
}

pub struct ArticleListHandler {}

impl ArticleListHandler {
    fn load_artiles() -> HttpResult<Vec<String>> {
        let dirs = read_dir("./static/text")?;
        Ok(dirs
            .filter_map(|dir| {
                if let Ok(dir) = dir {
                    return Some(dir);
                }
                None
            })
            .filter_map(|dir| {
                if let Ok(file_name) = dir.file_name().into_string()
                    && let Some(file_name) = file_name.split(".").next()
                {
                    return Some(file_name.to_owned());
                }
                None
            })
            .collect::<Vec<_>>())
    }
}

impl THttpMethodHandler for ArticleListHandler {
    fn post(
        &self,
        _request: ServerRequest,
        response: ServerResponse,
    ) -> Option<crate::HttpBoxedFuture<'_, ()>> {
        Some(Box::pin(async move {
            let articles = Self::load_artiles()?;
            response.lock().await.json(&ArticalList { articles }).await
        }))
    }
}
