use std::time;

use common::result::{HttpError, HttpResult};
use mini_runtime::{
    BoxedFutureWithError, create_server,
    web::{conn::SharedTcpConn, server::Server},
};

use crate::{
    Redirect,
    app::{
        articles::ArticleListHandler, completion::CompletionHandler, home::HomeHandler,
        html::HtmlGetterHandler,
    },
    route::add_route_handler,
    route_handler,
};

mod articles;
mod completion;
mod home;
mod html;
mod response;

fn init_route() {
    add_route_handler("".into(), Redirect::to("/home".to_owned()), None);
    add_route_handler("/".into(), Redirect::to("/home".to_owned()), None);
    add_route_handler("/home".into(), HomeHandler {}, None);
    add_route_handler(
        "/index".into(),
        HtmlGetterHandler::new("index".into()),
        None,
    );
    add_route_handler(
        "/hello".into(),
        HtmlGetterHandler::new("hello".into()),
        None,
    );
    add_route_handler(
        "/articles".into(),
        HtmlGetterHandler::new("articles".into()),
        None,
    );
    add_route_handler("/article_list".into(), ArticleListHandler {}, None);
    add_route_handler(
        "/completion".into(),
        CompletionHandler {},
        Some(time::Duration::from_secs(20).into()),
    );
}

pub fn create_app(
    ip: &str,
    port: usize,
    timeout: time::Duration,
) -> HttpResult<
    Server<HttpError, impl Fn(SharedTcpConn) -> BoxedFutureWithError<'static, (), HttpError>>,
> {
    let mut server = create_server!(ip, port, route_handler)?;
    server.update_timeout(|conn_timeout| {
        conn_timeout.update_timeout(timeout);
    });
    init_route();

    Ok(server)
}
