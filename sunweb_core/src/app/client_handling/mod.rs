use crate::app::server::files::get_static_file_content;
use crate::app::server::middleware::{Middleware, MiddlewareFn};
use crate::app::server::proxy::{Proxy, ProxySchema};
use crate::app::server::routes::Route;
use crate::app::server::routes::RouteType;
use crate::http_packet::header::connection::ConnectionType;
use crate::http_packet::header::content_types::ContentType;
use crate::http_packet::responses::status_code::StatusCode;
use crate::http_packet::responses::Response;
use crate::{HTTPMethod, HTTPRequest};
use log::{error, warn};
use rustls::ServerConfig;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;

enum Stream {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl Stream {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain(s) => s.read(buf).await,
            Stream::Tls(s) => s.read(buf).await,
        }
    }

    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            Stream::Plain(s) => s.write_all(buf).await,
            Stream::Tls(s) => s.write_all(buf).await,
        }
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Plain(s) => s.flush().await,
            Stream::Tls(s) => s.flush().await,
        }
    }
}

pub(crate) struct Client {
    stream: Stream,
    default_domain: String,
    middleware: Arc<Vec<Middleware>>,
    routes: Arc<Vec<Route>>,
}

impl Client {
    pub(crate) async fn new(
        stream: TcpStream,
        default_domain: String,
        middleware: Arc<Vec<Middleware>>,
        routes: Arc<Vec<Route>>,
        tls_config: Option<Arc<ServerConfig>>,
    ) -> Option<Self> {
        let stream = if let Some(tls_cfg) = tls_config {
            let acceptor = TlsAcceptor::from(tls_cfg);
            match acceptor.accept(stream).await {
                Ok(tls_stream) => Stream::Tls(tls_stream),
                Err(e) => {
                    warn!("TLS handshake failed: {e}");
                    return None;
                }
            }
        } else {
            Stream::Plain(stream)
        };

        Some(Self {
            stream,
            default_domain,
            middleware,
            routes,
        })
    }

    pub(crate) async fn handle(&mut self) -> Option<ConnectionType> {
        let raw_request = self.read_request().await?;

        let request = match HTTPRequest::parse(raw_request.as_ref()) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse HTTP request: {e}");
                return None;
            }
        };

        let connection = request.message.headers.connection.clone();
        let modified_request = self.apply_request_middleware(request.clone()).await;

        // For OPTIONS, skip routing and build a bare 204 for middleware to decorate
        let mut response = if *modified_request.method() == HTTPMethod::OPTIONS {
            Response::new(StatusCode::NoContent)
        } else {
            self.handle_routing(&modified_request).await
        };

        response = self.apply_response_middleware(modified_request.clone(), response).await;

        if *modified_request.method() == HTTPMethod::OPTIONS
            && response.get_header("Access-Control-Allow-Origin").is_none()
        {
            response = self.handle_routing(&modified_request).await;
            response = self.apply_response_middleware(modified_request, response).await;
        }

        self.send_response(response).await;
        Some(connection)
    }

    // ── Reading ───────────────────────────────────────────────────────────────

    async fn read_request(&mut self) -> Option<String> {
        let read_future = async {
            let mut buffer = Vec::with_capacity(2048);
            let mut chunk = [0u8; 1024];
            let headers_end_pos;

            loop {
                match self.stream.read(&mut chunk).await {
                    Ok(0) => return None,
                    Ok(n) => {
                        buffer.extend_from_slice(&chunk[..n]);
                        if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                            headers_end_pos = pos + 4;
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Socket read error: {e}");
                        return None;
                    }
                }
            }

            let content_length = parse_content_length(&buffer[..headers_end_pos]);
            while buffer.len() < headers_end_pos + content_length {
                match self.stream.read(&mut chunk).await {
                    Ok(0) => break,
                    Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                    Err(e) => {
                        warn!("Failed to read body: {e}");
                        break;
                    }
                }
            }

            Some(String::from_utf8_lossy(&buffer).into())
        };

        timeout(Duration::from_secs(5), read_future)
            .await
            .unwrap_or_else(|_| {
                warn!("Timeout reading request");
                None
            })
    }

    // ── Middleware ────────────────────────────────────────────────────────────

    async fn apply_request_middleware(&self, mut request: HTTPRequest) -> HTTPRequest {
        for mw in self.middleware.iter() {
            if mw.route != "*" && !request.path().starts_with(&mw.route) {
                continue;
            }
            if let MiddlewareFn::HTTPRequest(f) = &mw.f {
                f(&mut request);
            }
        }
        request
    }

    async fn apply_response_middleware(
        &self,
        mut request: HTTPRequest,
        mut response: Response,
    ) -> Response {
        for mw in self.middleware.iter() {
            if mw.route != "*" && !request.path().starts_with(&mw.route) {
                continue;
            }
            match &mw.f {
                MiddlewareFn::HTTPResponse(f) => f(&mut response),
                MiddlewareFn::HTTPResponseWithRoutes(f) => {
                    f(&mut request, &mut response, &self.routes)
                }
                MiddlewareFn::HTTPRequestResponse(f) => f(&mut request, &mut response),
                MiddlewareFn::HTTPResponseAsyncWithRoutes(f) => {
                    f(&mut request, &mut response, &self.routes).await
                }
                _ => {}
            }
        }
        response
    }

    // ── Sending ───────────────────────────────────────────────────────────────

    async fn send_response(&mut self, response: Response) {
        let bytes = response.to_bytes();
        if let Err(e) = self.stream.write_all(&bytes).await {
            warn!("Error writing response: {e}");
            return;
        }
        let _ = self.stream.flush().await;
    }

    // ── Routing ───────────────────────────────────────────────────────────────

    async fn handle_routing(&self, request: &HTTPRequest) -> Response {
        let host = request.host().unwrap_or_default();
        let path = request.path();
        let method = request.method();

        let domain_routes: Vec<&Route> = self
            .routes
            .iter()
            .filter(|r| r.domain == host || r.domain == self.default_domain)
            .collect();

        // 1. Static prefix match
        if let Some(route) = domain_routes
            .iter()
            .find(|r| r.route_type == RouteType::Static && path.starts_with(&r.path))
        {
            if let Some(folder) = &route.static_folder {
                return get_static_file_response(folder, request);
            }
        }

        // 2. Proxy prefix match
        if let Some(route) = domain_routes
            .iter()
            .find(|r| r.route_type == RouteType::Proxy && path.starts_with(&r.path))
        {
            if let Some(external) = &route.proxy_url {
                let prefix = route.path.clone();
                let external = external.clone();
                let request_clone = request.clone();
                return tokio::task::spawn_blocking(move || {
                    get_proxy_response(&prefix, &external, &request_clone)
                })
                .await
                .unwrap_or_else(|_| Response::internal_error());
            }
        }

        // 3. Exact match on path + method
        if let Some(route) = domain_routes
            .iter()
            .find(|r| r.route_type == RouteType::Standard && r.path == path)
        {
            if route.method != *method {
                return Response::method_not_allowed();
            }
            if let Some(f) = &route.handler {
                return f(request).await;
            }
        }

        Response::not_found()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn parse_content_length(header_bytes: &[u8]) -> usize {
    String::from_utf8_lossy(header_bytes)
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0)
}

fn get_static_file_response(folder: &String, request: &HTTPRequest) -> Response {
    let (content, content_type) = get_static_file_content(request.path(), folder);

    if content.is_empty() {
        return Response::not_found();
    }

    let mut response = Response::ok();
    response.set_body_string((*content).clone());
    response.set_content_type(content_type);
    response
}

fn get_proxy_response(prefix: &str, external: &str, request: &HTTPRequest) -> Response {
    let sub_path = request.path().strip_prefix(prefix).unwrap_or("/");
    let forward_path = format!("/{}", sub_path.trim_start_matches('/'));
    let full_url = format!("{}{}", external.trim_end_matches('/'), forward_path);

    let mut proxy = Proxy::new(full_url);
    if proxy.parse_url().is_none() {
        return Response::bad_gateway();
    }

    let Some(mut stream) = Proxy::connect_to_server(&proxy.host, proxy.port) else {
        return Response::bad_gateway();
    };

    let raw = match proxy.scheme {
        ProxySchema::HTTP => Proxy::send_http_request(&mut stream, &proxy.path, &proxy.host),
        ProxySchema::HTTPS => Proxy::send_https_request(&mut stream, &proxy.path, &proxy.host),
    };

    match raw {
        Some(bytes) => {
            let (body, content_type_str) = Proxy::parse_http_response_bytes(&bytes);
            let mut response = Response::new(StatusCode::Ok);
            response.set_body(body);
            if let Ok(ct) = ContentType::from_str(&content_type_str) {
                response.set_content_type(ct);
            }
            response.apply_cors_permissive();
            response
        }
        None => Response::bad_gateway(),
    }
}
