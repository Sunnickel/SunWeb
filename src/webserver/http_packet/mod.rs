//! Low-level HTTP message representation shared by requests and responses.
//!
//! This module is **internal**; users interact with the higher-level
//! [`HTTPRequest`](crate::webserver::requests::HTTPRequest) and
//! [`HTTPResponse`](crate::webserver::responses::HTTPResponse) types instead.
pub mod header;
use header::HTTPHeader;

/// An HTTP/1.1 message (request or response) without any semantic
/// interpretation.
///
/// Cloning is cheap: headers are reference-counted and the body is an
/// optional `Vec<u8>`.
#[derive(Clone, Debug)]
pub(crate) struct HTTPMessage {
    /// Protocol version as received on the wire, e.g. `"HTTP/1.1"`.
    pub http_version: String,
    /// Header map plus typed helpers.
    pub headers: HTTPHeader,
    /// Optional message body.
    pub body: Option<Vec<u8>>,
}

impl HTTPMessage {
    /// Creates a new message with the given version and headers.
    ///
    /// The body is initially empty (`None`).
    pub(crate) fn new(http_version: String, headers: HTTPHeader) -> Self {
        Self {
            http_version,
            headers,
            body: None,
        }
    }
}
