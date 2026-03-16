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
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct WebServer {
    pub(crate) config: ServerConfig,
    pub(crate) default_domain: String,
    pub(crate) middleware: Arc<Vec<Middleware>>,
    pub(crate) routes: Arc<Vec<Route>>,
}

impl WebServer {
    pub fn new(config: ServerConfig) -> WebServer {
        let default_domain = config.base_domain.clone();
        let mut middlewares = Vec::new();

        let logging_middleware = Middleware::new_request_response(None, Logger::log_request);
        let error_page_middleware =
            Middleware::new_response_async_with_routes(None, WebServer::error_page);

        middlewares.push(logging_middleware);
        middlewares.push(error_page_middleware);

        WebServer {
            config,
            default_domain,
            middleware: Arc::from(middlewares),
            routes: Arc::new(Vec::new()),
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
        let bind_addr = self.config.ip_as_string();
        let listener = TcpListener::bind(&bind_addr).await.unwrap();

        if self.config.using_https {
            info!("Server running on https://{bind_addr}/");
        } else {
            info!("Server running on http://{bind_addr}/");
        }

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let middleware = Arc::clone(&self.middleware);
                    let routes = Arc::clone(&self.routes);
                    let default_domain = self.default_domain.clone();
                    let tls_config = self.config.tls_config.clone();

                    tokio::spawn(async move {
                        let Some(mut client) =
                            Client::new(stream, default_domain, middleware, routes, tls_config)
                                .await
                        else {
                            return;
                        };

                        loop {
                            match client.handle().await {
                                Some(ConnectionType::KeepAlive) => continue,
                                Some(ConnectionType::Close) | None => break,
                                Some(other) => {
                                    error!("Unexpected connection type: {other}");
                                    break;
                                }
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