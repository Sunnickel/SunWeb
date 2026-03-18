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
use crate::Response;
use log::{error, info};
use rustls::ServerConfig as RustlsConfig;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct WebServer {
    pub(crate) config: ServerConfig,
    pub(crate) default_domain: String,
    pub(crate) middleware: Arc<Vec<Middleware>>,
    pub(crate) routes: Arc<Vec<Route>>,

    http_addr: Option<([u8; 4], u16)>,
    https_addr: Option<([u8; 4], u16, Arc<RustlsConfig>)>,
}

impl WebServer {
    pub fn new(
        config: ServerConfig,
        http_addr: Option<([u8; 4], u16)>,
        https_addr: Option<([u8; 4], u16, Arc<RustlsConfig>)>,
    ) -> WebServer {
        let default_domain = config.base_domain.clone();
        let middlewares = vec![
            Middleware::new_request_response(None, Logger::log_request),
            Middleware::new_response_async_with_routes(None, WebServer::error_page),
        ];
        WebServer {
            config,
            default_domain,
            middleware: Arc::from(middlewares),
            routes: Arc::new(Vec::new()),
            http_addr,
            https_addr,
        }
    }

    pub fn add_middleware(&mut self, middleware: Middleware) {
        Arc::make_mut(&mut self.middleware).push(middleware);
    }

    pub fn add_route(&mut self, route: Route) {
        Arc::make_mut(&mut self.routes).push(route);
    }

    pub fn start(&self) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(self.run());
    }

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

                        loop {
                            match client.handle().await {
                                Some(ConnectionType::KeepAlive) => continue,
                                _ => break,
                            }
                        }
                    });
                }
                Err(e) => eprintln!("Connection failed: {e}"),
            }
        }
    }

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

            if let Some(route) = routes
                .iter()
                .find(|r| r.route_type == RouteType::Error && r.status_code == status)
            {
                if let Some(handler) = &route.handler {
                    let error_resp = handler(request).await;
                    if let Some(body) = error_resp.body() {
                        if let Some(s) = body.as_string() {
                            response.set_body_string(s);
                        }
                    }
                    response.set_content_type(error_resp.content_type().clone());
                }
            }
        })
    }

    pub(crate) fn cors_check<'a>(
        request: &'a mut HTTPRequest,
        response: &'a mut Response,
        routes: &'a [Route],
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let status = response.status_code;

            if status.as_u16() < 400 {
                return;
            }

            if let Some(route) = routes
                .iter()
                .find(|r| r.route_type == RouteType::Error && r.status_code == status)
            {
                if let Some(handler) = &route.handler {
                    let error_resp = handler(request).await;
                    if let Some(body) = error_resp.body() {
                        if let Some(s) = body.as_string() {
                            response.set_body_string(s);
                        }
                    }
                    response.set_content_type(error_resp.content_type().clone());
                }
            }
        })
    }
}
