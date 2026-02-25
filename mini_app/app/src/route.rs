use std::{borrow::Cow, collections::HashMap, rc::Rc};

use lazy_static::lazy_static;
use mini_runtime::{ConnTimeout, UPSafeCell};

use crate::{HttpBoxedFuture, request::ServerRequest, response::ServerResponse};

lazy_static! {
    static ref HTTP_ROUTES: UPSafeCell<HashMap<String, Rc<HttpRouteHandle>>> =
        UPSafeCell::new(HashMap::new());
}

pub fn add_route_handler<H: THttpMethodHandler + 'static>(
    path: Cow<'_, str>,
    handler: H,
    timeout: Option<ConnTimeout>,
) {
    let mut http_routes = HTTP_ROUTES.exclusive_access();
    if http_routes.contains_key(path.as_ref()) {
        panic!("route {:?} already registered", path);
    }
    http_routes.insert(path.to_string(), HttpRouteHandle::new(handler, timeout));
}

pub fn select_route_handler(path: &str) -> Option<Rc<HttpRouteHandle>> {
    HTTP_ROUTES.exclusive_access().get(path).map(Rc::clone)
}

pub trait THttpMethodHandler {
    fn get(
        &self,
        _request: ServerRequest,
        _response: ServerResponse,
    ) -> Option<HttpBoxedFuture<'_, ()>> {
        None
    }

    fn post(
        &self,
        _request: ServerRequest,
        _response: ServerResponse,
    ) -> Option<HttpBoxedFuture<'_, ()>> {
        None
    }
}

pub struct HttpRouteHandle {
    handler: Box<dyn THttpMethodHandler>,
    timeout: Option<ConnTimeout>,
}

impl HttpRouteHandle {
    fn new<H: THttpMethodHandler + 'static>(handler: H, timeout: Option<ConnTimeout>) -> Rc<Self> {
        let handler: Box<dyn THttpMethodHandler> = Box::new(handler);
        Rc::new(Self { handler, timeout })
    }

    pub fn get_timeout(&self) -> Option<ConnTimeout> {
        self.timeout.clone()
    }

    pub fn get_handler(
        &self,
        request: ServerRequest,
        response: ServerResponse,
    ) -> Option<HttpBoxedFuture<'_, ()>> {
        match request.method() {
            common::HttpMethod::Get => self.handler.get(request, response),
            common::HttpMethod::Post => self.handler.post(request, response),
            _ => None,
        }
    }
}
