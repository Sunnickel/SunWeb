use crate::app::config::ServerConfig;
use crate::app::server::middleware::{Middleware, MiddlewareRegistration};
use crate::app::server::routes::{Route, RouteRegistration};
use crate::app::WebServer;
use crate::http_packet::header::http_method::HTTPMethod;
use crate::http_packet::responses::status_code::StatusCode;

pub struct AppBuilder {
    config: ServerConfig,
}

impl AppBuilder {
    pub fn new(host: [u8; 4], port: u16) -> Self {
        Self {
            config: ServerConfig::new(host, port),
        }
    }

    /// Set the base domain
    pub fn domain(mut self, domain: &str) -> Self {
        self.config = self.config.set_base_domain(domain.to_string());
        self
    }

    /// Add TLS certificate
    pub fn cert(mut self, key: &str, cert: &str) -> Self {
        self.config = self.config.add_cert(key.to_string(), cert.to_string());
        self
    }

    /// Build routes from inventory and start the server
    pub fn run(self) {
        let mut server = WebServer::new(self.config);

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
