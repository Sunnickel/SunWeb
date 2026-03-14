use crate::http_packet::header::http_method::HTTPMethod;
use crate::http_packet::requests::HTTPRequest;
use crate::http_packet::responses::Response;
use crate::http_packet::responses::status_code::StatusCode;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ── Handler type aliases ──────────────────────────────────────────────────────

pub type HandlerFuture<'a> = Pin<Box<dyn Future<Output = Response> + Send + 'a>>;
pub type HandlerFn = Arc<dyn for<'a> Fn(&'a HTTPRequest) -> HandlerFuture<'a> + Send + Sync>;

/// The type of a registered route, used during dispatch to select the correct handler.
#[derive(PartialEq, Clone, Debug)]
pub enum RouteType {
    Standard,
    Error,
    Static,
    Proxy,
}

/// A single registered route entry.
#[derive(Clone)]
pub struct Route {
    pub path: String,
    pub method: HTTPMethod,
    pub domain: String,
    pub route_type: RouteType,
    pub status_code: StatusCode,
    pub content: Option<String>,
    pub handler: Option<HandlerFn>,
    pub static_folder: Option<String>,
    pub proxy_url: Option<String>,
}

impl Route {
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

/// A route registration submitted via the `inventory` macro.
pub enum RouteRegistration {
    Custom {
        method: HTTPMethod,
        path: &'static str,
        handler: fn(&HTTPRequest) -> HandlerFuture,
    },
    Static {
        path: &'static str,
        folder: &'static str,
    },
    Error {
        status_code: u16,
        handler: fn(&HTTPRequest) -> HandlerFuture,
    },
    Proxy {
        path: &'static str,
        external: &'static str,
    },
}

inventory::collect!(RouteRegistration);