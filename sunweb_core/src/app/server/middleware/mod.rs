use crate::app::server::routes::Route;
use crate::http_packet::requests::HTTPRequest;
use crate::http_packet::responses::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Async middleware with access to the request, response, and route table.
/// Used for error pages and CORS checks.
pub type AsyncMiddlewareFn = Arc<
    dyn for<'a> Fn(
            &'a mut HTTPRequest,
            &'a mut Response,
            &'a [Route],
        ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
        + Send
        + Sync,
>;

/// Sync middleware with access to both the request and response.
pub type RequestResponseFn = Arc<dyn Fn(&mut HTTPRequest, &mut Response) + Send + Sync>;

/// Sync middleware with access to the request, response, and route table.
pub type ResponseWithRoutesFn =
    Arc<dyn Fn(&mut HTTPRequest, &mut Response, &[Route]) + Send + Sync>;

/// The concrete function stored inside a [`Middleware`], discriminated by
/// which arguments it receives.
pub enum MiddlewareFn {
    /// Runs before the handler — can mutate the request.
    HTTPRequest(Arc<dyn Fn(&mut HTTPRequest) + Send + Sync>),
    /// Runs after the handler — can mutate the response.
    HTTPResponse(Arc<dyn Fn(&mut Response) + Send + Sync>),
    /// Runs after the handler with access to both request and response.
    HTTPRequestResponse(RequestResponseFn),
    /// Sync variant with access to the route table (used for CORS checks etc.).
    HTTPResponseWithRoutes(ResponseWithRoutesFn),
    /// Async variant with access to the route table (used for error pages etc.).
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

/// A registered middleware entry combining a route scope and a handler function.
///
/// Use `"*"` as the route to apply the middleware to all requests, or supply a
/// path prefix to scope it (e.g. `"/api"`).
#[derive(Clone)]
pub struct Middleware {
    /// Route prefix this middleware applies to. `"*"` means all routes.
    pub(crate) route: String,
    /// The middleware function variant.
    pub(crate) f: MiddlewareFn,
}

impl Middleware {
    /// Creates a request middleware — runs before the route handler.
    pub fn new_request(route: Option<String>, f: fn(&mut HTTPRequest)) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPRequest(Arc::new(f)),
        }
    }

    /// Creates a response middleware — runs after the route handler.
    pub fn new_response(route: Option<String>, f: fn(&mut Response)) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPResponse(Arc::new(f)),
        }
    }

    /// Creates a middleware with access to both request and response.
    pub fn new_request_response(
        route: Option<String>,
        f: fn(&mut HTTPRequest, &mut Response),
    ) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPRequestResponse(Arc::new(f)),
        }
    }

    /// Creates a sync response middleware with access to the route table.
    pub fn new_response_with_routes(
        route: Option<String>,
        f: fn(&mut HTTPRequest, &mut Response, &[Route]),
    ) -> Middleware {
        Self {
            route: route.unwrap_or_else(|| "*".to_string()),
            f: MiddlewareFn::HTTPResponseWithRoutes(Arc::new(f)),
        }
    }

    /// Creates an async response middleware with access to the route table.
    ///
    /// Used internally for error page handling and CORS checks.
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

/// A middleware submitted at compile time via the `inventory` pattern.
///
/// Each `#[middleware]` macro call submits one of these variants. They are
/// collected and converted into [`Middleware`]s by [`AppBuilder::run`].
pub enum MiddlewareRegistration {
    /// Runs before the handler — receives a mutable request.
    Request {
        route: Option<&'static str>,
        handler: fn(&mut HTTPRequest),
    },
    /// Runs after the handler — receives a mutable response.
    Response {
        route: Option<&'static str>,
        handler: fn(&mut Response),
    },
    /// Runs after the handler — receives both mutable request and response.
    RequestResponse {
        route: Option<&'static str>,
        handler: fn(&mut HTTPRequest, &mut Response),
    },
}

inventory::collect!(MiddlewareRegistration);
