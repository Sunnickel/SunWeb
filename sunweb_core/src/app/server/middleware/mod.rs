use crate::app::server::routes::Route;
use crate::http_packet::requests::HTTPRequest;
use crate::http_packet::responses::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type AsyncMiddlewareFn = Arc<
    dyn for<'a> Fn(
            &'a mut HTTPRequest,
            &'a mut Response,
            &'a [Route],
        ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
        + Send
        + Sync,
>;

pub enum MiddlewareFn {
    HTTPRequest(Arc<dyn Fn(&mut HTTPRequest) + Send + Sync>),
    HTTPResponse(Arc<dyn Fn(&mut Response) + Send + Sync>),
    HTTPRequestResponse(Arc<dyn Fn(&mut HTTPRequest, &mut Response) + Send + Sync>),
    HTTPResponseWithRoutes(Arc<dyn Fn(&mut HTTPRequest, &mut Response, &[Route]) + Send + Sync>),
    HTTPResponseAsyncWithRoutes(AsyncMiddlewareFn),
}

impl Clone for MiddlewareFn {
    fn clone(&self) -> Self {
        match self {
            Self::HTTPRequest(f) => Self::HTTPRequest(Arc::clone(f)),
            Self::HTTPResponse(f) => Self::HTTPResponse(Arc::clone(f)),
            Self::HTTPRequestResponse(f) => Self::HTTPRequestResponse(Arc::clone(f)),
            Self::HTTPResponseWithRoutes(f) => Self::HTTPResponseWithRoutes(Arc::clone(f)),
            Self::HTTPResponseAsyncWithRoutes(f) => {
                Self::HTTPResponseAsyncWithRoutes(Arc::clone(f))
            }
        }
    }
}

#[derive(Clone)]
pub struct Middleware {
    pub(crate) route: String,
    pub(crate) f: MiddlewareFn,
}

impl Middleware {
    pub fn new_request(route: Option<String>, f: fn(&mut HTTPRequest)) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPRequest(Arc::new(f)),
        }
    }

    pub fn new_response(route: Option<String>, f: fn(&mut Response)) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPResponse(Arc::new(f)),
        }
    }

    pub fn new_response_with_routes(
        route: Option<String>,
        f: fn(&mut HTTPRequest, &mut Response, &[Route]),
    ) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPResponseWithRoutes(Arc::new(f)),
        }
    }

    pub fn new_request_response(
        route: Option<String>,
        f: fn(&mut HTTPRequest, &mut Response),
    ) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPRequestResponse(Arc::new(f)),
        }
    }

    pub fn new_response_async_with_routes<F>(route: Option<String>, f: F) -> Middleware
    where
        F: for<'a> Fn(
                &'a mut HTTPRequest,
                &'a mut Response,
                &'a [Route],
            ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPResponseAsyncWithRoutes(Arc::new(f)),
        }
    }
}

pub enum MiddlewareRegistration {
    Request {
        route: Option<&'static str>,
        handler: fn(&mut HTTPRequest),
    },
    Response {
        route: Option<&'static str>,
        handler: fn(&mut Response),
    },
    RequestResponse {
        route: Option<&'static str>,
        handler: fn(&mut HTTPRequest, &mut Response),
    },
}

inventory::collect!(MiddlewareRegistration);
