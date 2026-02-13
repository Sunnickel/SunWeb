//! Client Module - COMPLETE FIX with TLS Keep-Alive Support
//!
//! Critical fixes:
//! 1. TLS connections now properly read subsequent requests through the TLS connection
//! 2. Increased timeout to 5 seconds for keep-alive
//! 3. Proper state tracking to distinguish idle vs broken connections

use crate::webserver::Domain;
use crate::webserver::files::get_static_file_content;
use crate::webserver::http_packet::header::connection::ConnectionType;
use crate::webserver::http_packet::header::content_types::ContentType;
use crate::webserver::middleware::{Middleware, MiddlewareFn};
use crate::webserver::proxy::{Proxy, ProxySchema};
use crate::webserver::requests::HTTPRequest;
use crate::webserver::responses::HTTPResponse;
use crate::webserver::responses::status_code::StatusCode;
use crate::webserver::route::{Route, RouteType};
use log::{error, warn};
use rustls::{ServerConfig, ServerConnection};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(crate) struct Client {
    stream: TcpStream,
    domains: Arc<Mutex<HashMap<Domain, Arc<Mutex<Vec<Route>>>>>>,
    default_domain: Domain,
    middleware: Arc<Vec<Middleware>>,
    tls_config: Option<Arc<ServerConfig>>,
    tls_connection: Option<ServerConnection>,
}

impl Client {
    pub(crate) fn new(
        stream: TcpStream,
        domains: Arc<Mutex<HashMap<Domain, Arc<Mutex<Vec<Route>>>>>>,
        default_domain: Domain,
        middleware: Arc<Vec<Middleware>>,
        tls_config: Option<Arc<ServerConfig>>,
    ) -> Self {
        Self {
            stream,
            domains,
            default_domain,
            middleware,
            tls_config,
            tls_connection: None,
        }
    }

    pub(crate) fn handle(&mut self, i: u32) -> Option<ConnectionType> {
        let raw_request = if self.tls_config.is_some() {
            if i == 0 {
                self.handle_tls_connection()?
            } else {
                self.read_tls_request()?
            }
        } else {
            self.read_http_request()?
        };

        let request = match HTTPRequest::parse(raw_request.as_ref()) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse HTTP request: {e}");
                return None;
            }
        };

        let connection = request.headers().connection.clone();
        let modified_request = self.apply_request_middleware(request.clone());
        let response = self.handle_routing(modified_request);
        let final_response = self.apply_response_middleware(request, response);

        self.send_response(final_response);

        Some(connection)
    }

    /// Read HTTP request from plain TCP stream (non-TLS)
    fn read_http_request(&mut self) -> Option<String> {
        let _ = self
            .stream
            .set_read_timeout(Some(Duration::from_secs(5)));

        let mut buffer = Vec::with_capacity(2048);
        let mut chunk = [0u8; 1024];
        let mut headers_end_pos = 0;
        let mut first_read = true;

        loop {
            match self.stream.read(&mut chunk) {
                Ok(0) => {
                    return None;
                }
                Ok(n) => {
                    first_read = false;
                    buffer.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                        headers_end_pos = pos + 4;
                        break;
                    }
                }
                Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        if first_read {
                            return None;
                        }
                        warn!("Timeout while reading request headers");
                        return None;
                    }
                Err(e) => {
                    warn!("Socket read error: {e}");
                    return None;
                }
            }
        }

        let headers_str = String::from_utf8_lossy(&buffer[..headers_end_pos]);
        let content_length: usize = headers_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);

        while buffer.len() < headers_end_pos + content_length {
            match self.stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        warn!("Timeout reading request body");
                        break;
                    }
                Err(e) => {
                    warn!("Failed to read body: {e}");
                    break;
                }
            }
        }

        Some(String::from_utf8_lossy(&buffer).into())
    }

    /// NEW: Read from an already-established TLS connection (for i > 0)
    fn read_tls_request(&mut self) -> Option<String> {
        let conn = self.tls_connection.as_mut()?;

        let _ = self.stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buffer = Vec::with_capacity(2048);
        let mut chunk = [0u8; 2048];
        let mut headers_end_pos = 0;
        let mut first_read = true;

        // Read headers through TLS
        loop {
            // Process any pending TLS I/O
            if let Err(e) = conn.complete_io(&mut self.stream) {
                if first_read {
                    // Idle timeout on TLS - normal
                    return None;
                }
                warn!("TLS I/O error: {e}");
                return None;
            }

            match conn.reader().read(&mut chunk) {
                Ok(0) => {
                    // Connection closed
                    return None;
                }
                Ok(n) => {
                    first_read = false;
                    buffer.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                        headers_end_pos = pos + 4;
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if first_read {
                        // Idle timeout - close gracefully
                        return None;
                    }
                    // Partial read timeout
                    warn!("Timeout reading TLS request headers");
                    return None;
                }
                Err(e) => {
                    warn!("TLS read error: {e}");
                    return None;
                }
            }
        }

        // Parse Content-Length and read body if needed
        let headers_str = String::from_utf8_lossy(&buffer[..headers_end_pos]);
        let content_length: usize = headers_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);

        // Read body if present
        while buffer.len() < headers_end_pos + content_length {
            if conn.complete_io(&mut self.stream).is_err() {
                break;
            }

            match conn.reader().read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    warn!("Failed to read TLS body: {e}");
                    break;
                }
            }
        }

        Some(String::from_utf8_lossy(&buffer).into())
    }

    /// First TLS request: perform handshake and read initial request
    fn handle_tls_connection(&mut self) -> Option<String> {
        let tls_cfg = self.tls_config.as_ref()?.clone();
        let mut conn = self.perform_tls_handshake(tls_cfg)?;
        let buffer = self.read_tls_data(&mut conn)?;
        self.tls_connection = Some(conn);
        Some(String::from_utf8_lossy(&buffer).to_string())
    }

    fn perform_tls_handshake(&mut self, tls_config: Arc<ServerConfig>) -> Option<ServerConnection> {
        let mut conn = ServerConnection::new(tls_config).ok()?;
        while conn.is_handshaking() {
            if conn.complete_io(&mut self.stream).is_err() {
                return None;
            }
        }
        Some(conn)
    }

    fn read_tls_data(&mut self, conn: &mut ServerConnection) -> Option<Vec<u8>> {
        let _ = self.stream.set_nonblocking(true);
        let mut buffer = Vec::with_capacity(2048);
        let mut chunk = [0u8; 2048];

        loop {
            if conn.complete_io(&mut self.stream).is_err() {
                return None;
            }

            match conn.reader().read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => return None,
            }

            if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        if buffer.is_empty() {
            None
        } else {
            Some(buffer)
        }
    }

    fn apply_request_middleware(&self, mut request: HTTPRequest) -> HTTPRequest {
        for middleware in self.middleware.iter() {
            if middleware.route.as_str() != request.path && middleware.route.as_str() != "*" {
                continue;
            }
            if middleware.domain.as_str() != "*"
                && middleware.domain.as_str() != request.host().unwrap_or_default()
            {
                continue;
            }

            match &middleware.f {
                MiddlewareFn::HTTPRequest(func) => func(&mut request),
                MiddlewareFn::Both(req_func, _) => request = req_func(request),
                _ => {}
            }
        }
        request
    }

    fn apply_response_middleware(
        &self,
        mut original_request: HTTPRequest,
        mut response: HTTPResponse,
    ) -> HTTPResponse {
        for middleware in self.middleware.iter() {
            match &middleware.f {
                MiddlewareFn::HTTPResponse(func) => func(&mut response),
                MiddlewareFn::BothHTTPResponse(func) => {
                    response = func(&mut original_request, response)
                }
                MiddlewareFn::Both(_, res_func) => response = res_func(response),
                MiddlewareFn::HTTPResponseBothWithRoutes(func) => {
                    response = func(
                        &mut original_request,
                        response,
                        &*self
                            .domains
                            .lock()
                            .unwrap()
                            .get(&self.default_domain)
                            .unwrap()
                            .lock()
                            .unwrap(),
                    )
                }
                _ => {}
            }
        }
        response
    }

    fn send_response(&mut self, response: HTTPResponse) {
        let response_bytes = response.to_bytes();

        if let Some(conn) = &mut self.tls_connection {
            // Send response through TLS
            let chunk_size = 4096;
            let mut offset = 0;

            while offset < response_bytes.len() {
                let end = (offset + chunk_size).min(response_bytes.len());
                if conn
                    .writer()
                    .write_all(&response_bytes[offset..end])
                    .is_err()
                {
                    warn!("Error writing to TLS stream");
                    return;
                }
                if conn.complete_io(&mut self.stream).is_err() {
                    warn!("Error completing TLS write");
                    return;
                }
                offset = end;
            }

            // Ensure all data is flushed
            while conn.wants_write() {
                if conn.complete_io(&mut self.stream).is_err() {
                    warn!("Error flushing TLS write");
                    break;
                }
            }
        } else {
            // Plain HTTP
            let _ = self.stream.write_all(&response_bytes);
            let _ = self.stream.flush();
        }
    }

    fn handle_routing(&mut self, request: HTTPRequest) -> HTTPResponse {
        let host = request.host().unwrap_or_default();
        let current_domain = Domain::new(&host);

        let guard = self.domains.lock().unwrap();
        let routes_mutex = guard
            .get(&current_domain)
            .or_else(|| guard.get(&self.default_domain));

        let Some(routes_mutex) = routes_mutex else {
            return HTTPResponse::not_found();
        };

        let routes = routes_mutex.lock().unwrap();

        let matched_prefix = routes
            .iter()
            .filter(|r| request.path.starts_with(&r.route) && r.method == request.method)
            .max_by_key(|r| r.route.len());

        let route = match matched_prefix {
            Some(r) => r,
            None => return HTTPResponse::not_found(),
        };

        let exact = routes
            .iter()
            .find(|r| r.route == request.path)
            .unwrap_or(route);

        if exact.method != request.method {
            return HTTPResponse::method_not_allowed();
        }

        match exact.route_type {
            RouteType::Static => {
                if let Some(folder) = &exact.folder {
                    return get_static_file_response(folder, &request);
                }
            }
            RouteType::File => {
                if let Some(content) = &exact.content {
                    let mut response = HTTPResponse::new(exact.status_code);
                    response.set_body_string(content.to_string());
                    return response;
                }
            }
            RouteType::Custom => {
                if let Some(f) = &exact.f {
                    return catch_unwind(AssertUnwindSafe(|| f(request, &exact.domain)))
                        .unwrap_or_else(|_| HTTPResponse::internal_error());
                }
            }
            RouteType::Proxy => {
                if let Some(external) = &exact.external {
                    return get_proxy_route(&exact.route, external, &request);
                }
            }
            RouteType::Error => {
                if let Some(content) = &exact.content {
                    let mut response = HTTPResponse::new(exact.status_code);
                    response.set_body_string(content.to_string());
                    return response;
                }
            }
        }

        HTTPResponse::internal_error()
    }
}

fn get_proxy_route(prefix: &str, external: &String, request: &HTTPRequest) -> HTTPResponse {
    let path = format!(
        "{}/{}",
        prefix.trim_end_matches('/'),
        request.path.strip_prefix(prefix).unwrap_or("")
    );
    let joined = if external.ends_with('/') {
        format!("{}{}", external.trim_end_matches('/'), path)
    } else {
        format!("{}{}", external, path)
    };
    let mut proxy = Proxy::new(joined);

    if proxy.parse_url().is_none() {
        return HTTPResponse::bad_gateway();
    }

    let Some(mut stream) = Proxy::connect_to_server(&proxy.host, proxy.port) else {
        return HTTPResponse::bad_gateway();
    };

    let response_data = match proxy.scheme {
        ProxySchema::HTTP => Proxy::send_http_request(&mut stream, &proxy.path, &proxy.host),
        ProxySchema::HTTPS => Proxy::send_https_request(&mut stream, &proxy.path, &proxy.host),
    };

    if let Some(raw_response) = response_data {
        let (body_bytes, content_type) = Proxy::parse_http_response_bytes(&raw_response);
        let mut response = HTTPResponse::new(StatusCode::Ok);
        response.set_body(body_bytes);
        response.message.headers.content_type =
            ContentType::from_str(&*content_type).expect("Could not parse Content-Type");

        response.message.headers.apply_cors_permissive();

        return response;
    }

    HTTPResponse::bad_gateway()
}

fn get_static_file_response(folder: &String, request: &HTTPRequest) -> HTTPResponse {
    let (content, content_type) = get_static_file_content(&request.path, folder);

    if content.is_empty() {
        return HTTPResponse::not_found();
    }

    let mut response = HTTPResponse::ok();
    response.set_body_string(content.to_string());
    response.message.headers.content_type = content_type;
    response
}