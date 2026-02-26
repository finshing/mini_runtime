use std::time;

use common::{
    CT_EVENT_STREAM,
    dto::{BadResponse, SSEContent},
    result::HttpResult,
};
use mini_runtime::{sleep, web::conn::TcpConn};

use crate::{
    app::response::TextRandomSplitter,
    helper::load_file,
    request::ServerRequest,
    response::{SSEResponse, ServerResponse},
    route::THttpMethodHandler,
};

const DEFAULT_ARTICLE: &str = "一只特立独行的猪";

pub struct CompletionHandler {}

impl CompletionHandler {
    async fn completion(&self, request: ServerRequest, response: ServerResponse) -> HttpResult<()> {
        let article = request
            .url()
            .get_param("article")
            .unwrap_or(DEFAULT_ARTICLE.to_owned());

        let content = load_file(format!("./static/text/{}.txt", article).as_str());
        let content = match content {
            Ok(content) => content,
            Err(e) => {
                return response
                    .lock()
                    .await
                    .json(&BadResponse::new(format!("get article failed: {:?}", e)))
                    .await;
            }
        };
        let mut sse_response: SSEResponse<TcpConn> = response
            .lock()
            .await
            .update_header(|header| {
                header
                    .set_content_type(CT_EVENT_STREAM.into())
                    .set_cache_control("no-cache".into())
                    .set_connection("keep-alive".into());
            })
            .chunk()
            .await?
            .into();
        for chunk in TextRandomSplitter::new(content) {
            sse_response
                .write_event(SSEContent::resume(chunk).into())
                .flush()
                .await?;
            sleep(time::Duration::from_millis(50)).await;
        }
        sse_response
            .write_event(SSEContent::stop().into())
            .close()
            .await?;

        Ok(())
    }
}

impl THttpMethodHandler for CompletionHandler {
    fn get(
        &self,
        request: ServerRequest,
        response: ServerResponse,
    ) -> Option<crate::HttpBoxedFuture<'_, ()>> {
        Some(Box::pin(self.completion(request, response)))
    }
}
