#![feature(string_into_chars)]

use std::time;

use common::{
    HttpProtocol, HttpStatus,
    result::{HttpError, HttpResult},
};
use mini_runtime::{BoxedFutureWithError, variable_log, web::conn::SharedTcpConn};

use crate::{
    request::{_ServerRequest, ServerRequest},
    response::{ServerResponse, create_response},
    route::{THttpMethodHandler, select_route_handler},
};

pub mod app;
mod helper;
pub mod request;
pub mod response;
pub(crate) mod route;

pub type HttpBoxedFuture<'a, T> = BoxedFutureWithError<'a, T, HttpError>;

pub async fn route_handler(conn: SharedTcpConn) -> HttpResult<()> {
    let request = _ServerRequest::new(conn.clone().into()).await;
    let (path, response) = request
        .as_ref()
        .map(|req| {
            let path = req.url().path.to_owned();
            let protocol = req.protocol().clone();
            (path, create_response(conn.clone(), protocol))
        })
        .unwrap_or((
            String::new(),
            create_response(conn.clone(), HttpProtocol::default()),
        ));

    let start_at = time::Instant::now();
    log::info!("start request||url={}", path);
    let result = match request_route(conn, request, response.clone()).await {
        Err(e) => {
            log::warn!("response failed: {:?}", e);
            variable_log!(error @ internal_server_error(response.clone()).await, "internal server error")
        }
        _ => Ok(()),
    };
    log::info!(
        "end request||url={}||cost={:.2}ms||status={:?}",
        path,
        time::Instant::now().duration_since(start_at).as_micros() as f64 / 1000f64,
        result
    );

    Ok(())
}

async fn request_route(
    conn: SharedTcpConn,
    request: HttpResult<ServerRequest>,
    response: ServerResponse,
) -> HttpResult<()> {
    let request = match request {
        Ok(request) => request,
        Err(e) => {
            log::warn!("http server error: {:?}", e);
            return bad_request(response).await;
        }
    };

    let path = &request.url().path;
    let handler = match select_route_handler(path) {
        Some(handler) => handler,
        None => {
            log::warn!("not found handler for path '{}'", path);
            return not_found(response).await;
        }
    };

    if let Some(timeout) = handler.get_timeout() {
        conn.lock().await.set_timeout(timeout);
    }

    if let Some(method_handler) = handler.get_handler(request, response.clone()) {
        method_handler.await
    } else {
        method_not_allowed(response).await
    }
}

async fn bad_request(response: ServerResponse) -> HttpResult<()> {
    response
        .lock()
        .await
        .set_status(HttpStatus::BadRequest)
        .html_file("./static/error/bad_request.html")
        .await
}

async fn method_not_allowed(response: ServerResponse) -> HttpResult<()> {
    response
        .lock()
        .await
        .set_status(HttpStatus::MethodNotAllowed)
        .html_file("./static/error/method_not_allowed.html")
        .await
}

async fn not_found(response: ServerResponse) -> HttpResult<()> {
    response
        .lock()
        .await
        .set_status(HttpStatus::NotFound)
        .html_file("./static/error/not_found.html")
        .await
}

async fn internal_server_error(response: ServerResponse) -> HttpResult<()> {
    response
        .lock()
        .await
        .set_status(HttpStatus::InternalServerError)
        .html_file("./static/error/internal_server_error.html")
        .await
}

async fn redirect(response: ServerResponse, location: &str) -> HttpResult<()> {
    response
        .lock()
        .await
        .set_status(HttpStatus::MovedPermanently)
        .update_header(|header| {
            header.set_location(location.into());
        })
        .send(&[])
        .await
}

pub struct Redirect {
    location: String,
}

impl Redirect {
    pub fn to(location: String) -> Self {
        Self { location }
    }
}

impl THttpMethodHandler for Redirect {
    fn get(
        &self,
        _request: ServerRequest,
        response: ServerResponse,
    ) -> Option<HttpBoxedFuture<'_, ()>> {
        Some(Box::pin(redirect(response, &self.location)))
    }

    fn post(
        &self,
        _request: ServerRequest,
        response: ServerResponse,
    ) -> Option<HttpBoxedFuture<'_, ()>> {
        Some(Box::pin(redirect(response, &self.location)))
    }
}
