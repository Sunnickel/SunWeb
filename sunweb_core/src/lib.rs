//! Core types, traits, and utilities for the SunWeb framework.
//!
//! You should not depend on this crate directly — use [`sunweb`] instead,
//! which re-exports everything from here.

pub mod app;
pub mod http_packet;
mod logger;

pub use crate::app::builder::AppBuilder;
pub use crate::app::server::files::get_file_content;
pub use crate::app::server::middleware::MiddlewareRegistration;
pub use crate::app::server::routes::RouteRegistration;
pub use crate::http_packet::header::http_method::HTTPMethod;
pub use crate::http_packet::requests::HTTPRequest;
pub use crate::http_packet::responses::*;
pub use crate::logger::Logger;

/// Parses a `"host:port"` string into a raw `([u8; 4], u16)` address.
///
/// Accepts dotted-decimal IPv4 addresses. Pass the result directly to
/// [`AppBuilder::bind`].
///
/// # Panics
///
/// Panics with a descriptive message if the format is invalid — bad host
/// segments, a non-numeric port, or a missing `:` separator.
///
/// # Example
///
/// ```rust,ignore
/// use sunweb::parse_addr;
///
/// let addr = parse_addr("0.0.0.0:8080");  // ([0, 0, 0, 0], 8080)
/// let addr = parse_addr("127.0.0.1:3000"); // ([127, 0, 0, 1], 3000)
/// ```
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
