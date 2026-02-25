use crate::route::THttpMethodHandler;

pub struct HtmlGetterHandler {
    file_name: String,
}

impl HtmlGetterHandler {
    pub fn new(file_name: String) -> Self {
        Self { file_name }
    }
}

impl THttpMethodHandler for HtmlGetterHandler {
    fn get(
        &self,
        _request: crate::request::ServerRequest,
        response: crate::response::ServerResponse,
    ) -> Option<crate::HttpBoxedFuture<'_, ()>> {
        Some(Box::pin(async move {
            response
                .lock()
                .await
                .html_file(format!("./static/html/{}.html", self.file_name).as_str())
                .await
        }))
    }
}
