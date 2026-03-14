use crate::app::server::routes::Route;
use crate::http_packet::requests::HTTPRequest;
use crate::http_packet::responses::Response;

#[derive(Clone)]
pub enum MiddlewareFn {
    HTTPRequest(fn(&mut HTTPRequest)),
    HTTPResponse(fn(&mut Response)),
    HTTPResponseWithRoutes(fn(&mut HTTPRequest, &mut Response, &[Route])),
    HTTPRequestResponse(fn(&mut HTTPRequest, &mut Response)),
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
            f: MiddlewareFn::HTTPRequest(f),
        }
    }

    pub fn new_response(route: Option<String>, f: fn(&mut Response)) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPResponse(f),
        }
    }

    pub fn new_response_with_routes(
        route: Option<String>,
        f: fn(&mut HTTPRequest, &mut Response, &[Route]),
    ) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPResponseWithRoutes(f),
        }
    }

    pub fn new_request_response(route: Option<String>, f: fn(&mut HTTPRequest, &mut Response)) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPRequestResponse(f),
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