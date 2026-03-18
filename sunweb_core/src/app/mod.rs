pub mod builder;
mod client_handling;
pub mod config;
pub mod server;

use crate::app::client_handling::Client;
use crate::app::config::ServerConfig;
use crate::app::server::middleware::Middleware;
use crate::app::server::routes::{Route, RouteType};
use crate::http_packet::header::connection::ConnectionType;
use crate::http_packet::requests::HTTPRequest;
use crate::logger::Logger;
use crate::status_code::StatusCode;
use crate::{HTTPMethod, Response};
use log::info;
use rustls::ServerConfig as RustlsConfig;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;

/// The core server runtime — owns the listener sockets, routes, and middleware
/// and drives the async accept loop.
///
/// You should not construct this directly. Use [`AppBuilder`] instead, which
/// configures and starts a `WebServer` for you.
pub struct WebServer {
    pub(crate) default_domain: String,
    pub(crate) middleware: Arc<Vec<Middleware>>,
    pub(crate) routes: Arc<Vec<Route>>,

    http_addr: Option<([u8; 4], u16)>,
    https_addr: Option<([u8; 4], u16, Arc<RustlsConfig>)>,
}

impl WebServer {
    /// Creates a new `WebServer` with the given config and optional HTTP/HTTPS
    /// listener addresses.
    ///
    /// Registers the built-in request logger and error page middleware
    /// automatically. Called internally by [`AppBuilder::build`].
    pub fn new(
        config: ServerConfig,
        http_addr: Option<([u8; 4], u16)>,
        https_addr: Option<([u8; 4], u16, Arc<RustlsConfig>)>,
    ) -> WebServer {
        let default_domain = config.base_domain.clone();
        let middlewares = vec![
            Middleware::new_request_response(None, Logger::log_request),
            Middleware::new_response_async_with_routes(None, WebServer::error_page),
            Middleware::new_response_async_with_routes(None, WebServer::cors_check),
        ];
        WebServer {
            default_domain,
            middleware: Arc::from(middlewares),
            routes: Arc::new(Vec::new()),
            http_addr,
            https_addr,
        }
    }

    /// Appends a middleware to the chain.
    pub fn add_middleware(&mut self, middleware: Middleware) {
        Arc::make_mut(&mut self.middleware).push(middleware);
    }

    /// Registers a route with the server.
    pub fn add_route(&mut self, route: Route) {
        Arc::make_mut(&mut self.routes).push(route);
    }

    /// Blocks the current thread and starts the Tokio runtime, then begins
    /// accepting connections.
    ///
    /// This is the final call in the builder chain — it does not return until
    /// the server is shut down.
    pub fn start(&self) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(self.run());
    }

    /// Binds the configured listener sockets and spawns an accept loop task
    /// for each one (HTTP and/or HTTPS).
    async fn run(&self) {
        let mut tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();
        let https_port = self.https_addr.as_ref().map(|(_, port, _)| *port);

        if let Some((host, port)) = self.http_addr {
            let addr = format!("{}.{}.{}.{}:{}", host[0], host[1], host[2], host[3], port);
            let listener = TcpListener::bind(&addr).await.unwrap();
            info!("HTTP  listening on http://{addr}");

            let middleware = Arc::clone(&self.middleware);
            let routes = Arc::clone(&self.routes);
            let domain = self.default_domain.clone();

            tasks.push(tokio::spawn(async move {
                Self::accept_loop(listener, domain, middleware, routes, None, https_port).await;
            }));
        }

        if let Some((host, port, tls)) = self.https_addr.clone() {
            let addr = format!("{}.{}.{}.{}:{}", host[0], host[1], host[2], host[3], port);
            let listener = TcpListener::bind(&addr).await.unwrap();
            info!("HTTPS listening on https://{addr}");

            let middleware = Arc::clone(&self.middleware);
            let routes = Arc::clone(&self.routes);
            let domain = self.default_domain.clone();

            tasks.push(tokio::spawn(async move {
                Self::accept_loop(listener, domain, middleware, routes, Some(tls), None).await;
            }));
        }

        assert!(!tasks.is_empty(), "No listeners configured");
        futures::future::join_all(tasks).await;
    }

    /// Continuously accepts incoming TCP connections from `listener` and
    /// spawns a task for each one.
    ///
    /// Each connection runs a keep-alive loop via [`Client::handle`] until the
    /// connection is closed or an error occurs.
    async fn accept_loop(
        listener: TcpListener,
        default_domain: String,
        middleware: Arc<Vec<Middleware>>,
        routes: Arc<Vec<Route>>,
        tls_config: Option<Arc<RustlsConfig>>,
        https_port: Option<u16>,
    ) {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let middleware = Arc::clone(&middleware);
                    let routes = Arc::clone(&routes);
                    let domain = default_domain.clone();
                    let tls = tls_config.clone();

                    tokio::spawn(async move {
                        let Some(mut client) =
                            Client::new(stream, domain, middleware, routes, tls, https_port).await
                        else {
                            return;
                        };

                        while let Some(ConnectionType::KeepAlive) = client.handle().await {}
                    });
                }
                Err(e) => eprintln!("Connection failed: {e}"),
            }
        }
    }

    /// Built-in response middleware that replaces the response body with a
    /// custom error page when a matching [`RouteType::Error`] route is registered
    /// for the response's status code.
    ///
    /// Only runs for responses with a 4xx or 5xx status code. Registered
    /// automatically in [`WebServer::new`].
    pub(crate) fn error_page<'a>(
        request: &'a mut HTTPRequest,
        response: &'a mut Response,
        routes: &'a [Route],
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let status = response.status_code;

            if status.as_u16() < 400 {
                return;
            }

            if let Some(handler) = routes
                .iter()
                .find(|r| r.route_type == RouteType::Error && r.status_code == status)
                .and_then(|route| route.handler.as_ref())
            {
                let error_resp = handler(request).await;
                if let Some(s) = error_resp.body().and_then(|b| b.as_string()) {
                    response.set_body_string(s);
                }
                response.set_content_type(error_resp.content_type().clone());
            }
        })
    }

    /// CORS preflight middleware — not yet wired into the default middleware
    /// chain.
    ///
    /// Intended to validate `Origin` headers and inject the appropriate
    /// `Access-Control-*` response headers.
    pub(crate) fn cors_check<'a>(
        request: &'a mut HTTPRequest,
        response: &'a mut Response,
        _routes: &'a [Route],
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let method = request.method();
            let origin = request.message.headers.get_header("Origin");

            let Some(origin) = origin else {
                return;
            };

            if method == HTTPMethod::OPTIONS {
                let requested_method = request
                    .message
                    .headers
                    .get_header("Access-Control-Request-Method")
                    .unwrap_or_default();

                let requested_headers = request
                    .message
                    .headers
                    .get_header("Access-Control-Request-Headers")
                    .unwrap_or_default();

                response.set_cors_origin(&origin);
                response.add_header("Access-Control-Allow-Methods", &requested_method);
                response.add_header("Access-Control-Allow-Headers", &requested_headers);
                response.set_cors_max_age(86400);
                response.status_code = StatusCode::NoContent;
            } else {
                response.set_cors_origin(&origin);
            }
        })
    }
}
