#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod http_packet;
pub mod app;
mod logger;

pub use crate::app::builder::AppBuilder;
pub use crate::http_packet::header::http_method::HTTPMethod;
pub use crate::http_packet::requests::HTTPRequest;
pub use crate::http_packet::responses::*;
pub use crate::app::server::routes::RouteRegistration;
pub use crate::app::server::middleware::MiddlewareRegistration;
pub use crate::app::server::files::get_file_content;

/// Parses `"host:port"` into `([u8; 4], u16)` for `AppBuilder::new`.
///
/// Accepts dotted-decimal IPv4 (`"127.0.0.1:8080"`) or the special
/// string `"0.0.0.0:port"`.
///
/// # Panics
///
/// Panics with a clear message if the format is invalid.
pub fn parse_addr(addr: &str) -> ([u8; 4], u16) {
    let (host_str, port_str) = addr
        .rsplit_once(':')
        .unwrap_or_else(|| panic!("Invalid addr `{addr}` — expected `host:port`"));

    let port: u16 = port_str
        .parse()
        .unwrap_or_else(|_| panic!("Invalid port `{port_str}` in addr `{addr}`"));

    let parts: Vec<u8> = host_str
        .split('.')
        .map(|seg| {
            seg.parse::<u8>()
                .unwrap_or_else(|_| panic!("Invalid host segment `{seg}` in addr `{addr}`"))
        })
        .collect();

    let host = match parts.as_slice() {
        [a, b, c, d] => [*a, *b, *c, *d],
        _ => panic!("Host `{host_str}` must be an IPv4 address (4 octets)"),
    };

    (host, port)
}
