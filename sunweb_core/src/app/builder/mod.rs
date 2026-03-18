use crate::app::config::ServerConfig;
use crate::app::server::middleware::{Middleware, MiddlewareRegistration};
use crate::app::server::routes::{Route, RouteRegistration};
use crate::app::WebServer;
use crate::http_packet::header::http_method::HTTPMethod;
use crate::http_packet::responses::status_code::StatusCode;
use crate::parse_addr;

pub struct AppBuilder {
    http_addr: Option<([u8; 4], u16)>,
    https_config: Option<ServerConfig>,
    domain: Option<String>,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            http_addr: None,
            https_config: None,
            domain: None,
        }
    }

    pub fn http(mut self, addr: &str) -> Self {
        self.http_addr = Some(parse_addr(addr));
        self
    }

    pub fn https(mut self, addr: &str) -> Self {
        let (host, port) = parse_addr(addr);
        self.https_config = Some(ServerConfig::new(host, port));
        self
    }

    pub fn cert(mut self, key: &str, cert: &str) -> Self {
        let cfg = self.https_config
            .take()
            .expect(".cert() called without .https()");
        self.https_config = Some(cfg.add_cert(key.to_string(), cert.to_string()));
        self
    }

    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = Some(domain.to_string());
        self
    }

    /// Build routes from inventory and start the server
    pub fn run(self) {
        let base_addr = self.https_config
            .as_ref()
            .map(|c| (c.host, c.port))
            .or(self.http_addr)
            .expect("At least one of .http() or .https() must be called");

        let mut base_config = ServerConfig::new(base_addr.0, base_addr.1);
        if let Some(domain) = self.domain {
            base_config = base_config.set_base_domain(domain);
        }
        if let Some(ref https_cfg) = self.https_config {
            if let Some(tls) = https_cfg.tls_config.clone() {
                base_config.tls_config = Some(tls);
                base_config.using_https = true;
            }
        }

        let mut server = WebServer::new(
            base_config,
            self.http_addr,
            self.https_config.map(|c| (c.host, c.port, c.tls_config.unwrap())),
        );

        for route in inventory::iter::<RouteRegistration> {
            match route {
                RouteRegistration::Custom {
                    method,
                    path,
                    handler,
                } => {
                    server.add_route(Route::new_custom(
                        path.to_string(),
                        method.clone(),
                        StatusCode::Ok,
                        server.default_domain.clone(),
                        *handler,
                    ));
                }
                RouteRegistration::Static { path, folder } => {
                    server.add_route(Route::new_static(
                        path.to_string(),
                        HTTPMethod::GET,
                        StatusCode::Ok,
                        server.default_domain.clone(),
                        folder.to_string(),
                    ));
                }
                RouteRegistration::Error {
                    status_code,
                    handler,
                } => {
                    server.add_route(Route::new_error(
                        HTTPMethod::GET,
                        server.default_domain.clone(),
                        StatusCode::from(*status_code),
                        *handler,
                    ));
                }
                RouteRegistration::Proxy { path, external } => {
                    server.add_route(Route::new_proxy(
                        path.to_string(),
                        HTTPMethod::GET,
                        server.default_domain.clone(),
                        StatusCode::Ok,
                        external.to_string(),
                    ));
                }
            }
        }

        for middleware in inventory::iter::<MiddlewareRegistration> {
            let mw = match middleware {
                MiddlewareRegistration::Request { route, handler } => {
                    Middleware::new_request(route.map(|s| s.to_string()), *handler)
                }
                MiddlewareRegistration::Response { route, handler } => {
                    Middleware::new_response(route.map(|s| s.to_string()), *handler)
                }
                MiddlewareRegistration::RequestResponse { route, handler } => {
                    Middleware::new_request_response(route.map(|s| s.to_string()), *handler)
                }
            };
            server.add_middleware(mw);
        }

        server.start();
    }
}
