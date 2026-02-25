use crate::{request::ServerRequest, response::ServerResponse, route::THttpMethodHandler};

const HOME_CONTENT: &str = r#"
<html>
    <head>
        <title>Home</title>
    </head>
    <body>
        <h3>This is home</h3>
    </body
</html>
"#;

pub(crate) struct HomeHandler {}

impl THttpMethodHandler for HomeHandler {
    fn get(
        &self,
        _request: ServerRequest,
        response: ServerResponse,
    ) -> Option<crate::HttpBoxedFuture<'_, ()>> {
        Some(Box::pin(async move {
            let mut resp = response.lock().await;
            resp.html(HOME_CONTENT).await?;
            Ok(())
        }))
    }
}
