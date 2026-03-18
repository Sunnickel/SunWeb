use crate::http_packet::header::http_method::HTTPMethod;
use crate::http_packet::requests::HTTPRequest;
use crate::http_packet::responses::Response;
use crate::http_packet::responses::status_code::StatusCode;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ── Handler type aliases ──────────────────────────────────────────────────────

/// A boxed, heap-allocated async future returned by a route handler.
pub type HandlerFuture<'a> = Pin<Box<dyn Future<Output = Response> + Send + 'a>>;

/// A type-erased, cloneable route handler function.
pub type HandlerFn = Arc<dyn for<'a> Fn(&'a HTTPRequest) -> HandlerFuture<'a> + Send + Sync>;

/// Discriminates between the four kinds of registered routes during dispatch.
#[derive(PartialEq, Clone, Debug)]
pub enum RouteType {
    /// A regular method + path handler registered with `#[get]`, `#[post]`, etc.
    Standard,
    /// A custom error page handler registered with `#[error_page(N)]`.
    Error,
    /// A static file folder registered with `#[static_files]`.
    Static,
    /// A reverse-proxy route registered with `#[proxy]`.
    Proxy,
}

/// A single registered route, owned by the [`WebServer`] at startup.
///
/// Routes are constructed via the `new_*` factory methods and should not be
/// built manually.
#[derive(Clone)]
pub struct Route {
    /// URL path prefix this route matches on.
    pub path: String,
    /// HTTP method this route accepts.
    pub method: HTTPMethod,
    /// Domain this route is scoped to.
    pub domain: String,
    /// The kind of route — controls which dispatch branch is used.
    pub route_type: RouteType,
    /// Status code associated with this route (used for error routes).
    pub status_code: StatusCode,
    pub content: Option<String>,
    /// Async handler called when this route matches.
    pub handler: Option<HandlerFn>,
    /// Local folder path for static file routes.
    pub static_folder: Option<String>,
    /// External base URL for proxy routes.
    pub proxy_url: Option<String>,
}

impl Route {
    /// Creates a standard method + path route with an async handler.
    pub fn new_custom(
        path: String,
        method: HTTPMethod,
        status_code: StatusCode,
        domain: String,
        handler: impl Fn(&HTTPRequest) -> HandlerFuture + Send + Sync + 'static,
    ) -> Self {
        Route {
            path,
            method,
            domain,
            route_type: RouteType::Standard,
            status_code,
            content: None,
            handler: Some(Arc::new(handler)),
            static_folder: None,
            proxy_url: None,
        }
    }

    /// Creates a static file route that serves files from `folder` under `path`.
    pub fn new_static(
        path: String,
        method: HTTPMethod,
        status_code: StatusCode,
        domain: String,
        folder: String,
    ) -> Self {
        Route {
            path,
            method,
            domain,
            route_type: RouteType::Static,
            status_code,
            content: None,
            handler: None,
            static_folder: Some(folder),
            proxy_url: None,
        }
    }

    /// Creates a custom error page route for the given status code.
    pub fn new_error(
        method: HTTPMethod,
        domain: String,
        status_code: StatusCode,
        handler: impl Fn(&HTTPRequest) -> HandlerFuture + Send + Sync + 'static,
    ) -> Self {
        Route {
            path: String::new(),
            method,
            domain,
            route_type: RouteType::Error,
            status_code,
            content: None,
            handler: Some(Arc::new(handler)),
            static_folder: None,
            proxy_url: None,
        }
    }

    /// Creates a reverse-proxy route that forwards requests to `external`.
    ///
    /// Trailing slashes on `external` are stripped automatically.
    pub fn new_proxy(
        path: String,
        method: HTTPMethod,
        domain: String,
        status_code: StatusCode,
        external: String,
    ) -> Self {
        Route {
            path,
            method,
            domain,
            route_type: RouteType::Proxy,
            status_code,
            content: None,
            handler: None,
            static_folder: None,
            proxy_url: Some(external.trim_end_matches('/').to_string()),
        }
    }
}

/// A route submitted at compile time via the `inventory` pattern.
///
/// Each `#[get]`, `#[post]`, `#[static_files]`, `#[error_page]`, and
/// `#[proxy]` macro call submits one of these variants. They are collected
/// and converted into [`Route`]s by [`AppBuilder::run`].
pub enum RouteRegistration {
    /// A standard HTTP method + path handler.
    Custom {
        method: HTTPMethod,
        path: &'static str,
        handler: fn(&HTTPRequest) -> HandlerFuture,
    },
    /// A static file folder served under a URL prefix.
    Static {
        path: &'static str,
        folder: &'static str,
    },
    /// A custom error page for a specific HTTP status code.
    Error {
        status_code: u16,
        handler: fn(&HTTPRequest) -> HandlerFuture,
    },
    /// A reverse-proxy route forwarding to an external URL.
    Proxy {
        path: &'static str,
        external: &'static str,
    },
}

inventory::collect!(RouteRegistration);
