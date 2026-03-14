//! HTTP `Connection` header value.
//!
//! The only registered tokens in RFC 9110 are `close` and `keep-alive`;
//! `upgrade` is used by the WebSocket handshake.  Any other token is
//! preserved verbatim via [`ConnectionType::Other`](ConnectionType::Other).

use std::fmt;

/// A single connection directive that can appear in the HTTP `Connection`
/// header.
///
/// ```
/// use http_packet::header::connection::ConnectionType;
///
/// let ct = ConnectionType::KeepAlive;
/// assert_eq!(ct.to_string(), "keep-alive");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConnectionType {
    /// `keep-alive`
    KeepAlive,
    /// `close`
    Close,
    /// `upgrade`
    Upgrade,
    /// Any other token (e.g. `TE`, `http2-settings`, …)
    Other(String),
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::KeepAlive => "keep-alive",
            Self::Close => "close",
            Self::Upgrade => "upgrade",
            Self::Other(text) => text,
        })
    }
}
