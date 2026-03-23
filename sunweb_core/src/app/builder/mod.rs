use crate::app::config::ServerConfig;
use crate::app::server::middleware::Middleware;
use crate::app::server::routes::Route;
use crate::app::WebServer;
use crate::status_code::StatusCode;
use crate::{parse_addr, HTTPMethod, MiddlewareRegistration, RouteRegistration};

/// Fluent builder for configuring and starting a [`WebServer`].
///
/// Obtain one via `MyApp::builder()` after deriving [`App`] on your app struct.
///
/// # Example
/// ```rust,ignore
/// use sunweb::App;
///
/// #[derive(App)]
/// struct MyApp;
///
/// #[tokio::main]
/// async fn main() {
///     MyApp::builder()
///         .http("0.0.0.0:8080")
///         .run();
/// }
/// ```
///
/// With HTTPS:
/// ```rust,ignore
/// MyApp::builder()
///     .http("0.0.0.0:8080")   // optional HTTP (redirects to HTTPS)
///     .https("0.0.0.0:8443")
///     .cert("/path/to/key.pem", "/path/to/cert.pem")
///     .domain("example.com")
///     .run();
/// ```
pub struct AppBuilder {
    http_addr: Option<([u8; 4], u16)>,
    https_config: Option<ServerConfig>,
    domain: Option<String>,
    http2: bool,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    /// Creates a new builder with no listeners configured.
    pub fn new() -> Self {
        Self {
            http_addr: None,
            https_config: None,
            domain: None,
            http2: false,
        }
    }

    /// Binds an HTTP listener on `addr` (e.g. `"0.0.0.0:8080"`).
    ///
    /// When used alongside `.https()`, HTTP connections are automatically
    /// redirected to the HTTPS port.
    pub fn http(mut self, addr: &str) -> Self {
        self.http_addr = Some(parse_addr(addr));
        self
    }

    /// Binds an HTTPS listener on `addr`. Must be followed by `.cert()`.
    pub fn https(mut self, addr: &str) -> Self {
        let (host, port) = parse_addr(addr);
        self.https_config = Some(ServerConfig::new(host, port));
        self
    }

    pub fn http2(mut self) -> Self {
        self.http2 = true;
        self
    }

    /// Loads the TLS private key and certificate for the HTTPS listener.
    ///
    /// # Panics
    /// Panics if called before `.https()`.
    pub fn cert(mut self, key: &str, cert: &str) -> Self {
        if let Some(cfg) = self.https_config.take() {
            self.https_config = Some(cfg.add_cert(key.to_string(), cert.to_string(), self.http2));
        }
        self
    }

    /// Sets the default domain used for route matching.
    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = Some(domain.to_string());
        self
    }

    /// Collects all routes and middleware registered via `inventory`, builds
    /// the [`WebServer`], and blocks until the server shuts down.
    ///
    /// # Panics
    /// Panics if neither `.http()` nor `.https()` was called.
    pub fn run(self) {
        // Validate: at least one listener must be configured
        assert!(
            self.http_addr.is_some() || self.https_config.is_some(),
            "At least one of .http() or .https() must be called before .run()"
        );

        // Validate: .https() is required if .cert() or .http2() was used
        if self.http2 {
            assert!(
                self.https_config.is_some(),
                ".http2() requires .https() to be configured"
            );
        }

        // Validate: .https() requires .cert()
        if let Some(ref https_cfg) = self.https_config {
            assert!(
                https_cfg.tls_config.is_some(),
                ".https() was called but no certificate was provided — call .cert() before .run()"
            );
        }

        let base_addr = self
            .https_config
            .as_ref()
            .map(|c| (c.host, c.port))
            .or(self.http_addr)
            .unwrap(); // safe: already asserted above

        let mut base_config = ServerConfig::new(base_addr.0, base_addr.1);
        if let Some(domain) = self.domain {
            base_config = base_config.set_base_domain(domain);
        }
        if let Some(ref https_cfg) = self.https_config
            && let Some(tls) = https_cfg.tls_config.clone()
        {
            base_config.tls_config = Some(tls);
            base_config.using_https = true;
        }

        let mut server = WebServer::new(
            base_config,
            self.http2,
            self.http_addr,
            self.https_config
                .map(|c| (c.host, c.port, c.tls_config.unwrap())),
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
