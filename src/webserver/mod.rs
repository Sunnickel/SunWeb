//!
//! It is designed to allow creating route handlers for static files, dynamic content,
//! proxying, and custom error pages. Most functions operate on `WebServer` instances
//! and internal utility types like `Domain` and `Route`.
//!
//! # Example
//!
//! ```rust
//! use webserver::{WebServer, ServerConfig, Domain};
//! use webserver::route::HTTPMethod;
//! use webserver::responses::StatusCode;
//!
//! // Create a basic configuration
//! let config = ServerConfig::new("127.0.0.1", 8080, "example.com");
//! let mut server = WebServer::new(config);
//!
//! // Add a custom route
//! server.add_custom_route("/api", HTTPMethod::GET, |_req, _domain| {
//!     // Return a simple HTTP response
//!     webserver::responses::HTTPResponse::new(200, "Hello API".to_string())
//! }, StatusCode::Ok, None);
//!
//! // Add a file route
//! server.add_route_file("/about", HTTPMethod::GET, "./static/about.html", StatusCode::Ok, None);
//!
//! // Add a static folder route
//! server.add_static_route("/assets", HTTPMethod::GET, "./static/assets", StatusCode::Ok, None);
//! ```
mod client_handling;
pub(crate) mod files;
pub mod http_packet;
pub(crate) mod logger;
pub(crate) mod middleware;
mod proxy;
pub mod requests;
pub mod responses;
pub mod route;
pub(crate) mod server_config;

use crate::webserver::client_handling::Client;
use crate::webserver::files::get_file_content;
use crate::webserver::middleware::Middleware;
use crate::webserver::route::{HTTPMethod, Route, RouteType};
pub use crate::webserver::server_config::ServerConfig;

use crate::webserver::http_packet::header::connection::ConnectionType;
use crate::webserver::logger::Logger;
use crate::webserver::requests::HTTPRequest;
use crate::webserver::responses::{HTTPResponse, StatusCode};
use log::{error, info};
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// Represents a domain name used for routing.
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct Domain {
    /// The domain name as a string.
    pub name: String,
}

impl Domain {
    /// Creates a new `Domain` instance.
    ///
    /// # Arguments
    ///
    /// * `name` - The domain name as a string slice.
    ///
    /// # Returns
    ///
    /// A new `Domain` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use webserver::Domain;
    /// let domain = Domain::new("api");
    /// assert_eq!(domain.as_str(), "api");
    /// ```
    pub fn new(name: &str) -> Domain {
        Self {
            name: name.to_string(),
        }
    }

    /// Returns the domain name as a `String`.
    pub fn as_str(&self) -> String {
        self.name.clone()
    }
}

/// The main web server structure.
///
/// Handles configuration, domains, routes, and middleware.
pub struct WebServer {
    /// Server configuration including IP, port, and TLS settings.
    pub(crate) config: ServerConfig,
    /// Map of domains to their respective routing configurations.
    pub(crate) domains: Arc<Mutex<HashMap<Domain, Arc<Mutex<Vec<Route>>>>>>,
    /// The default domain used for subdomain generation.
    pub(crate) default_domain: Domain,
    /// List of middleware functions to apply to requests/responses.
    pub(crate) middleware: Arc<Vec<Middleware>>,
}

impl WebServer {
    /// Creates a new `WebServer` instance with the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The `ServerConfig` object containing server settings.
    ///
    /// # Returns
    ///
    /// A new `WebServer` instance with default middleware registered.
    ///
    /// # Example
    ///
    /// ```rust
    /// use webserver::{WebServer, ServerConfig};
    /// let config = ServerConfig::new("127.0.0.1", 8080, "example.com");
    /// let server = WebServer::new(config);
    /// ```
    pub fn new(config: ServerConfig) -> WebServer {
        let mut domains = HashMap::new();
        let default_domain = Domain::new(&config.base_domain);
        domains.insert(default_domain.clone(), Arc::new(Mutex::new(Vec::new())));
        let mut middlewares = Vec::new();

        let logging_start_middleware =
            Middleware::new_request(None, None, Logger::log_request_start);
        let logging_end_middleware = Middleware::new_response(None, None, Logger::log_request_end);
        let error_page_middleware =
            Middleware::new_response_both_w_routes(None, None, Self::error_page);

        middlewares.push(logging_start_middleware);
        middlewares.push(logging_end_middleware);
        middlewares.push(error_page_middleware);

        WebServer {
            config,
            domains: Arc::new(Mutex::new(domains)),
            default_domain,
            middleware: Arc::from(middlewares),
        }
    }

    /// Starts the web server.
    ///
    /// This will bind the server to the configured IP and port, spawn threads to handle
    /// incoming connections, and apply registered middleware to all requests.
    ///
    /// # Panics
    ///
    /// This function will panic if the server fails to bind to the IP/port.
    pub fn start(&self) {
        let bind_addr = self.config.ip_as_string();
        let listener = TcpListener::bind(&bind_addr).unwrap();
        if self.config.using_https {
            info!("Server running on https://{bind_addr}/");
        } else {
            info!(
                "Server running on http://{bind_addr}/",
                bind_addr = bind_addr
            );
        }
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let domains = Arc::clone(&self.domains);
                    let middleware = Arc::clone(&self.middleware);
                    let default_domain = self.default_domain.clone();
                    let tls_config = self.config.tls_config.clone();

                    thread::spawn(move || {
                        let mut client =
                            Client::new(stream, domains, default_domain, middleware, tls_config);

                        let mut i = 0;
                        loop {
                            match client.handle(i) {
                                Some(connection_type) => match connection_type {
                                    ConnectionType::KeepAlive => {
                                        i += 1;
                                        continue;
                                    }
                                    ConnectionType::Close => {
                                        break;
                                    }
                                    _ => {
                                        error!("Connection error: {connection_type}");
                                        break;
                                    }
                                },
                                None => {
                                    continue
                                }
                            }
                        }
                    });
                }
                Err(e) => eprintln!("Connection failed: {e}"),
            }
        }
    }

    /// Adds a subdomain router for the specified domain.
    ///
    /// # Arguments
    ///
    /// * `domain` - A reference to the `Domain` to register.
    ///
    /// # Example
    ///
    /// ```rust
    /// use webserver::{WebServer, ServerConfig, Domain};
    /// let config = ServerConfig::new("127.0.0.1", 8080, "example.com");
    /// let mut server = WebServer::new(config);
    /// let domain = Domain::new("api");
    /// server.add_subdomain_router(&domain);
    /// ```
    pub fn add_subdomain_router(&mut self, domain: &Domain) {
        let mut guard = self.domains.lock().unwrap();
        let domain_str = format!(
            "{}.{}",
            domain.name.to_lowercase(),
            self.default_domain.name
        );
        guard
            .entry(Domain::new(&*domain_str))
            .or_insert_with(|| Arc::new(Mutex::new(Vec::new())));
    }

    /// Adds a file-based route to the server.
    ///
    /// # Arguments
    ///
    /// * `route` - The URL path to match (e.g., "/about").
    /// * `method` - HTTP method for the route.
    /// * `file_path` - Local path to the file to serve.
    /// * `response_codes` - Status code to respond with.
    /// * `domain` - Optional `Domain`; defaults to the default domain.
    pub fn add_route_file(
        &mut self,
        route: &str,
        method: HTTPMethod,
        file_path: &str,
        response_codes: StatusCode,
        domain: Option<&Domain>,
    ) -> &mut Self {
        let domain = domain
            .cloned()
            .unwrap_or_else(|| self.default_domain.clone());

        let content = get_file_content(&PathBuf::from(file_path));

        {
            let mut guard = self.domains.lock().unwrap();
            let domain_routes = guard
                .entry(domain.clone())
                .or_insert_with(|| Arc::new(Mutex::new(Vec::new())));

            let mut routes = domain_routes.lock().unwrap();
            routes.push(Route::new_file(
                route.to_string(),
                method,
                response_codes,
                domain,
                content,
            ));
        }

        self
    }

    /// Adds a static folder route (serving all files in a directory).
    ///
    /// # Arguments
    ///
    /// * `route` - The URL path to match (e.g., "/static").
    /// * `method` - HTTP method.
    /// * `folder` - Local folder path containing static files.
    /// * `response_codes` - Status code for successful responses.
    /// * `domain` - Optional `Domain`; defaults to the default domain.
    pub fn add_static_route(
        &mut self,
        route: &str,
        method: HTTPMethod,
        folder: &str,
        response_codes: StatusCode,
        domain: Option<&Domain>,
    ) -> &mut Self {
        let domain = domain
            .cloned()
            .unwrap_or_else(|| self.default_domain.clone());

        let folder_path = PathBuf::from(folder);
        if !folder_path.exists() {
            error!("Static route file does not exist");
        }

        {
            let mut guard = self.domains.lock().unwrap();
            let domain_routes = guard
                .entry(domain.clone())
                .or_insert_with(|| Arc::new(Mutex::new(Vec::new())));

            let mut routes = domain_routes.lock().unwrap();
            routes.push(Route::new_static(
                route.to_string(),
                method,
                response_codes,
                domain,
                String::from(folder),
            ));
        }
        self
    }

    /// Adds a custom route with a handler function.
    ///
    /// # Example
    ///
    /// ```rust
    /// use webserver::{WebServer, ServerConfig, Domain, HTTPRequest, HTTPResponse};
    /// use webserver::route::HTTPMethod;
    /// use webserver::responses::StatusCode;
    ///
    /// let config = ServerConfig::new("127.0.0.1", 8080, "example.com");
    /// let mut server = WebServer::new(config);
    /// server.add_custom_route("/api", HTTPMethod::GET, |_request, _domain| {
    ///     HTTPResponse::new(200, "Hello API".to_string())
    /// }, StatusCode::Ok, None);
    /// ```
    pub fn add_custom_route(
        &mut self,
        route: &str,
        method: HTTPMethod,
        f: impl Fn(HTTPRequest, &Domain) -> HTTPResponse + Send + Sync + 'static,
        response_codes: StatusCode,
        domain: Option<&Domain>,
    ) -> &mut Self {
        let domain = domain
            .cloned()
            .unwrap_or_else(|| self.default_domain.clone());
        {
            let mut guard = self.domains.lock().unwrap();
            let domain_routes = guard
                .entry(domain.clone())
                .or_insert_with(|| Arc::new(Mutex::new(Vec::new())));

            let mut routes = domain_routes.lock().unwrap();
            routes.push(Route::new_custom(
                route.to_string(),
                method,
                response_codes,
                domain,
                f,
            ));
        }
        self
    }

    /// Adds a custom error page route.
    ///
    /// This allows replacing default error pages (like 404 Not Found or 500 Internal Server Error)
    /// with custom HTML content from a local file. The provided file will be served whenever
    /// the specified status code occurs.
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the HTML file to serve as the error page.
    /// * `response_codes` - Status code that this error page corresponds to (e.g., `StatusCode::NotFound`).
    /// * `domain` - Optional domain reference; if `None`, the default domain is used.
    pub fn add_error_route(
        &mut self,
        file: &str,
        response_codes: StatusCode,
        domain: Option<&Domain>,
    ) -> &mut Self {
        let domain = domain
            .cloned()
            .unwrap_or_else(|| self.default_domain.clone());

        let content = get_file_content(&PathBuf::from(file));

        {
            let mut guard = self.domains.lock().unwrap();
            let domain_routes = guard
                .entry(domain.clone())
                .or_insert_with(|| Arc::new(Mutex::new(Vec::new())));

            let mut routes = domain_routes.lock().unwrap();
            routes.push(Route::new_error(
                HTTPMethod::GET,
                domain,
                response_codes,
                content,
            ));
        }
        self
    }

    /// Adds a proxy route to forward requests to an external service.
    ///
    /// Incoming requests matching `route` will be forwarded to `external` URL.
    /// This is useful for integrating microservices or external APIs.
    ///
    /// # Arguments
    ///
    /// * `route` - URL path to match (e.g., "/api").
    /// * `external` - Full external URL to forward the request to (e.g., "https://api.example.com").
    /// * `response_codes` - Status code to respond with for successful proxying.
    /// * `domain` - Optional domain reference; if `None`, the default domain is used.
    pub fn add_proxy_route(
        &mut self,
        route: &str,
        external: &str,
        response_codes: StatusCode,
        domain: Option<&Domain>,
    ) -> &mut Self {
        let domain = domain
            .cloned()
            .unwrap_or_else(|| self.default_domain.clone());

        {
            let mut guard = self.domains.lock().unwrap();
            let domain_routes = guard
                .entry(domain.clone())
                .or_insert_with(|| Arc::new(Mutex::new(Vec::new())));

            let mut routes = domain_routes.lock().unwrap();
            routes.push(Route::new_proxy(
                route.to_string(),
                HTTPMethod::GET,
                domain,
                response_codes,
                external.to_string(),
            ));
        }
        self
    }

    /// Internal middleware function for handling error pages.
    ///
    /// This function is used internally to override default error responses
    /// with custom error pages if a matching route is registered.
    ///
    /// # Arguments
    ///
    /// * `_request` - Mutable reference to the incoming `HTTPRequest`.
    /// * `response` - The `HTTPResponse` generated for the request.
    /// * `routes` - All registered routes for the current domain.
    ///
    /// # Returns
    ///
    /// Returns the original `HTTPResponse` or a custom response if a matching error page route exists.
    ///
    /// # Note
    ///
    /// This function is `pub(crate)` and intended for internal server logic; users generally
    /// do not call this directly.
    pub(crate) fn error_page(
        _request: &mut HTTPRequest,
        response: HTTPResponse,
        routes: &[Route],
    ) -> HTTPResponse {
        let status_code = response.status_code;

        if let Some(route) = routes
            .iter()
            .find(|x| x.route_type == RouteType::Error && x.status_code == status_code)
        {
            if let Some(content) = &route.content {
                let mut response = HTTPResponse::new(status_code);
                response.set_body_string(content.to_string());
            }
        }

        response
    }
}
