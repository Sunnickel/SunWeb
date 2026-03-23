pub mod h2;

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
/// Abstracts over plain TCP and TLS streams so the rest of the client
/// handling code doesn't need to branch on the transport.
enum Stream {
    Plain(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
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

    async fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain(s) => s.read_exact(buf).await,
            Stream::Tls(s) => s.read_exact(buf).await,
        }
    }
}

/// Represents a single connected client and drives the request/response
/// lifecycle for that connection.
///
/// One `Client` is spawned per accepted TCP connection. It reads requests,
/// applies middleware, dispatches to the matching route, and writes the
/// response back — repeating for keep-alive connections.
pub(crate) struct Client {
    stream: Stream,
    default_domain: String,
    middleware: Arc<Vec<Middleware>>,
    routes: Arc<Vec<Route>>,
    /// Whether this connection was accepted over TLS.
    is_https: bool,
    /// The HTTPS port, used to build HTTP→HTTPS redirect URLs.
    https_port: Option<u16>,
    http2: bool,
}

impl Client {
    /// Accepts a TCP stream and performs a TLS handshake if the first bytes
    /// look like a TLS `ClientHello`. Returns `None` if the handshake fails.
    pub(crate) async fn new(
        stream: TcpStream,
        default_domain: String,
        middleware: Arc<Vec<Middleware>>,
        routes: Arc<Vec<Route>>,
        tls_config: Option<Arc<ServerConfig>>,
        https_port: Option<u16>,
        http2: bool,
    ) -> Option<Client> {
        let mut buf = [0u8; 3];
        stream.peek(&mut buf).await.expect("Couldn't peek stream");

        let is_tls = buf[0] == 0x16 && buf[1] == 0x03;
        let stream = if is_tls {
            if let Some(tls_cfg) = tls_config {
                let acceptor = TlsAcceptor::from(tls_cfg);
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => Stream::Tls(Box::new(tls_stream)),
                    Err(e) => {
                        warn!("TLS handshake failed: {e}");
                        return None;
                    }
                }
            } else {
                return None;
            }
        } else {
            Stream::Plain(stream)
        };

        Some(Self {
            stream,
            default_domain,
            middleware,
            routes,
            is_https: is_tls,
            https_port,
            http2,
        })
    }

    /// Dispatches the connection to the correct HTTP version handler.
    ///
    /// For TLS connections, ALPN is checked first. If the client negotiated
    /// `h2`, we log that HTTP/2 is not yet implemented and close the
    /// connection gracefully. Everything else (including plain HTTP) falls
    /// through to [`Client::handle_http1`].
    ///
    /// Returns the `Connection` header value, or `None` to close.
    pub(crate) async fn handle(&mut self) -> Option<ConnectionType> {
        if self.http2 {
            if let Stream::Tls(ref tls_stream) = self.stream {
                let alpn = tls_stream.get_ref().1.alpn_protocol().map(|p| p.to_vec());

                match alpn.as_deref() {
                    Some(b"h2") => {
                        self.handle_http2().await.expect("TODO: panic message");
                        return None;
                    }
                    Some(b"http/1.1") | None => {}
                    Some(other) => {
                        warn!(
                            "Unknown ALPN protocol {:?}; falling back to HTTP/1.1",
                            String::from_utf8_lossy(other)
                        );
                    }
                }
            }
        }

        self.handle_http1().await
    }

    /// Reads one request, runs the middleware chain, dispatches it, and writes
    /// the response. Returns the `Connection` header value so the caller can
    /// decide whether to keep the connection alive.
    async fn handle_http1(&mut self) -> Option<ConnectionType> {
        let raw_request = self.read_request().await?;

        let request = match HTTPRequest::parse(raw_request.as_ref()) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse HTTP request: {e}");
                return None;
            }
        };

        // Redirect plain HTTP to HTTPS when both listeners are active.
        if !self.is_https
            && let Some(https_port) = self.https_port
        {
            let host = request.host()?;
            let bare_host = host.split(':').next().unwrap_or(&host);
            let location = if https_port == 443 {
                format!("https://{}{}", bare_host, request.path())
            } else {
                format!("https://{}:{}{}", bare_host, https_port, request.path())
            };

            let mut response = Response::new(StatusCode::MovedPermanently);
            response.add_header("Location", &location);
            self.send_response(response).await;
            return None;
        }

        let connection = request.message.headers.connection.clone();
        let modified_request = self.apply_request_middleware(request.clone()).await;

        let mut response = if *modified_request.method() == HTTPMethod::OPTIONS {
            Response::new(StatusCode::NoContent)
        } else {
            self.handle_routing(&modified_request).await
        };

        response = self
            .apply_response_middleware(modified_request.clone(), response)
            .await;

        // If OPTIONS produced no CORS headers, fall through to the route handler.
        if *modified_request.method() == HTTPMethod::OPTIONS
            && response.get_header("Access-Control-Allow-Origin").is_none()
        {
            response = self.handle_routing(&modified_request).await;
            response = self
                .apply_response_middleware(modified_request, response)
                .await;
        }

        self.send_response(response).await;
        Some(connection)
    }

    // ── Reading ───────────────────────────────────────────────────────────────

    /// Reads a complete HTTP request from the stream, including the body if
    /// `Content-Length` is set. Times out after 5 seconds.
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

    /// Runs all request-phase middleware in registration order, scoped by
    /// route prefix.
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

    /// Runs all response-phase middleware in registration order, scoped by
    /// route prefix.
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

    /// Serializes `response` and writes it to the stream.
    async fn send_response(&mut self, response: Response) {
        let bytes = response.to_bytes();
        if let Err(e) = self.stream.write_all(&bytes).await {
            warn!("Error writing response: {e}");
            return;
        }
        let _ = self.stream.flush().await;
    }

    // ── Routing ───────────────────────────────────────────────────────────────

    /// Dispatches the request to the first matching route in priority order:
    /// static prefix → proxy prefix → exact method + path match.
    ///
    /// Returns `404 Not Found` if no route matches, or `405 Method Not
    /// Allowed` if the path matches but the method does not.
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
        if let Some(folder) = domain_routes
            .iter()
            .find(|r| r.route_type == RouteType::Static && path.starts_with(&r.path))
            .and_then(|r| r.static_folder.as_ref())
        {
            return get_static_file_response(folder, request);
        }

        // 2. Proxy prefix match
        if let Some((prefix, external)) = domain_routes
            .iter()
            .find(|r| r.route_type == RouteType::Proxy && path.starts_with(&r.path))
            .and_then(|r| r.proxy_url.as_ref().map(|u| (r.path.clone(), u.clone())))
        {
            let request_clone = request.clone();
            return tokio::task::spawn_blocking(move || {
                get_proxy_response(&prefix, &external, &request_clone)
            })
                .await
                .unwrap_or_else(|_| Response::internal_error());
        }

        // 3. Standard routes — exact matches first, then :param patterns
        let standard_routes: Vec<&Route> = domain_routes
            .iter()
            .copied()
            .filter(|r| r.route_type == RouteType::Standard)
            .collect();

        let matched = standard_routes
            .iter()
            .find(|r| r.path == path)
            .map(|r| (*r, vec![]))
            .or_else(|| {
                standard_routes
                    .iter()
                    .find_map(|r| match_path(&r.path, path).map(|params| (*r, params)))
            });

        if let Some((route, params)) = matched {
            if route.method != *method {
                return Response::method_not_allowed();
            }
            if let Some(f) = &route.handler {
                let mut req_with_params = request.clone();
                for (key, value) in params {
                    req_with_params.set_path_param(key.into(), value);
                }
                return f(&req_with_params).await;
            }
        }

        Response::not_found()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extracts the `Content-Length` value from raw header bytes, returning `0`
/// if the header is absent or unparseable.
fn parse_content_length(header_bytes: &[u8]) -> usize {
    String::from_utf8_lossy(header_bytes)
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0)
}

/// Serves a file from `folder` matching the request path, or returns `404`.
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

/// Forwards the request to an external server via HTTP or HTTPS proxy and
/// returns the proxied response, or `502 Bad Gateway` on failure.
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
        ProxySchema::Http => Proxy::send_http_request(&mut stream, &proxy.path, &proxy.host),
        ProxySchema::Https => Proxy::send_https_request(&mut stream, &proxy.path, &proxy.host),
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

/// Checks whether a route pattern matches a request path, and if so returns
/// the extracted param values keyed by their name.
/// e.g. pattern "/:id/posts/:page" against "/42/posts/3" → [("id","42"),("page","3")]
fn match_path<'a>(pattern: &'a str, path: &'a str) -> Option<Vec<(&'a str, String)>> {
    let pattern_segs: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if pattern_segs.len() != path_segs.len() {
        return None;
    }

    let mut params = Vec::new();
    for (pat, val) in pattern_segs.iter().zip(path_segs.iter()) {
        if let Some(name) = pat.strip_prefix(':') {
            params.push((name, val.to_string()));
        } else if pat != val {
            return None;
        }
    }

    Some(params)
}