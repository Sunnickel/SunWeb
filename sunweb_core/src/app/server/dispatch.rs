use std::str::FromStr;
use crate::app::server::files::get_static_file_content;
use crate::app::server::proxy::{Proxy, ProxySchema};
use crate::app::server::routes::{HandlerFuture, Route, RouteType};
use crate::http_packet::header::content_types::ContentType;
use crate::http_packet::requests::HTTPRequest;
use crate::http_packet::responses::Response;
use crate::http_packet::responses::status_code::StatusCode;

/// Dispatches an incoming request to the first matching route.
pub fn dispatch<'a>(request: &'a HTTPRequest, routes: &[Route]) -> HandlerFuture<'a> {
    let path = request.path();
    let method = request.method();

    // 1. Static prefix match
    if let Some(route) = routes.iter().find(|r| {
        r.route_type == RouteType::Static && path.starts_with(&r.path)
    }) {
        let response = handle_static(request, route);
        return Box::pin(async move { response });
    }

    // 2. Proxy prefix match
    if let Some(route) = routes.iter().find(|r| {
        r.route_type == RouteType::Proxy && path.starts_with(&r.path)
    }) {
        // Proxy is blocking — capture what we need and run in spawn_blocking
        let upstream_base = route.proxy_url.clone().unwrap();
        let route_path = route.path.clone();
        let request_clone = request.clone();

        return Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                handle_proxy_sync(&request_clone, &route_path, &upstream_base)
            })
                .await
                .unwrap_or_else(|_| Response::internal_error())
        });
    }

    // 3. Exact match — method and path must both match
    if let Some(route) = routes.iter().find(|r| {
        r.route_type == RouteType::Standard && r.path == path && r.method == *method
    }) {
        if let Some(handler) = &route.handler {
            return handler(request);
        }
    }

    // 4. No match
    Box::pin(async { Response::not_found() })
}

/// Resolves a static file from disk and returns its content with the correct MIME type.
fn handle_static(request: &HTTPRequest, route: &Route) -> Response {
    let folder = route.static_folder.as_ref().unwrap();

    let sub_path = request
        .path()
        .strip_prefix(&route.path)
        .unwrap_or("/");
    let routable = format!("/{}", sub_path.trim_start_matches('/'));

    let (content, content_type) = get_static_file_content(&routable, folder);

    if content.is_empty() {
        return Response::not_found();
    }

    let mut response = Response::ok();
    response.set_content_type(content_type);
    response.set_body_string((*content).clone());
    response
}

/// Forwards a request to an upstream server and returns its response.
fn handle_proxy_sync(request: &HTTPRequest, route_path: &str, upstream_base: &str) -> Response {
    let sub_path = request
        .path()
        .strip_prefix(route_path)
        .unwrap_or("/");
    let forward_path = format!("/{}", sub_path.trim_start_matches('/'));
    let full_url = format!("{}{}", upstream_base, forward_path);

    let mut proxy = Proxy::new(full_url);
    if proxy.parse_url().is_none() {
        return Response::bad_gateway();
    }

    let raw = match proxy.scheme {
        ProxySchema::HTTP => {
            let Some(mut stream) = Proxy::connect_to_server(&proxy.host, proxy.port) else {
                return Response::bad_gateway();
            };
            Proxy::send_http_request(&mut stream, &proxy.path, &proxy.host)
        }
        ProxySchema::HTTPS => {
            let Some(mut stream) = Proxy::connect_to_server(&proxy.host, proxy.port) else {
                return Response::bad_gateway();
            };
            Proxy::send_https_request(&mut stream, &proxy.path, &proxy.host)
        }
    };

    match raw {
        Some(bytes) => {
            let (body, content_type_str) = Proxy::parse_http_response_bytes(&bytes);
            let mut response = Response::ok();
            if let Ok(ct) = ContentType::from_str(&content_type_str) {
                response.set_content_type(ct);
            }
            response.set_body(body);
            response
        }
        None => Response::bad_gateway(),
    }
}