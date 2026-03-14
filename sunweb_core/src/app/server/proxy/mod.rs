//! Tiny HTTP **and** HTTPS forward-proxy helper built on `rustls`.
//!
//! The crate is **not** a full-featured proxy; it only performs:
//! 1. URL parsing (`Proxy`)
//! 2. one-shot `GET` requests
//! 3. minimal HTTP/1.1 response parsing (headers + chunked or `Content-Length` body)
//!
//! Timeouts are hard-coded to 5 s.  Keep-alive is **not** supported.

use log::warn;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use rustls_native_certs::load_native_certs;
use rustls_pki_types::ServerName;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// Transport scheme inferred from the URL.
#[derive(Debug)]
pub(crate) enum ProxySchema {
    /// Plain-text HTTP (port 80 by default).
    HTTP,
    /// TLS-wrapped HTTPS (port 443 by default).
    HTTPS,
}

/// A very small HTTP/HTTPS client that can execute one `GET` request.
pub(crate) struct Proxy {
    /// Original URL supplied by the caller.
    url: String,
    /// Extracted host name (not including port).
    pub(crate) host: String,
    /// Port that will be connected to.
    pub(crate) port: u16,
    /// Path + query + fragment (always starts with `/`).
    pub(crate) path: String,
    /// Whether HTTPS or plain HTTP will be used.
    pub(crate) scheme: ProxySchema,
}

impl Proxy {
    /// Creates an **uninitialised** proxy.
    ///
    /// You **must** call [`parse_url`](Self::parse_url) before using any other
    /// method; all other functions assume a successfully parsed URL.
    ///
    /// # Example
    ///
    /// ```
    /// let mut p = Proxy::new("https://example.com/api".into());
    /// assert!(p.parse_url().is_some());
    /// ```
    pub(crate) fn new(url: String) -> Self {
        Self {
            url,
            host: String::new(),
            port: 0u16,
            path: String::new(),
            scheme: ProxySchema::HTTPS,
        }
    }

    /// Splits the stored URL into `(scheme, host, port, path)`.
    ///
    /// Returns `None` for malformed URLs or unsupported schemes (only `http`
    /// and `https` are recognised).  On success, the fields `host`, `port`,
    /// `path`, and `scheme` are updated in place.
    pub(crate) fn parse_url(&mut self) -> Option<()> {
        let mut parts = self.url.splitn(2, "://");
        let scheme = parts.next()?.to_lowercase();
        let rest = parts.next()?;

        let (host_port, path) = match rest.split_once('/') {
            Some((hp, p)) => (hp, format!("/{}", p)),
            None => (rest, "/".to_string()),
        };

        let (host, port) = match host_port.split_once(':') {
            Some((h, p)) => {
                let port_num = p.parse::<u16>().ok()?;
                (h.to_string(), port_num)
            }
            None => {
                let default_port = match scheme.as_str() {
                    "https" => 443,
                    "http" => 80,
                    _ => return None,
                };
                (host_port.to_string(), default_port)
            }
        };

        self.scheme = match scheme.as_str() {
            "https" => ProxySchema::HTTPS,
            "http" => ProxySchema::HTTP,
            _ => return None,
        };
        self.host = host;
        self.port = port;
        self.path = path;

        Some(())
    }

    /// Opens a **TCP** connection to `(host, port)` with a 5 s read/write timeout.
    ///
    /// Logs a warning on failure.  The returned stream is ready for plain HTTP
    /// **or** can be wrapped in TLS for HTTPS.
    pub(crate) fn connect_to_server(host: &str, port: u16) -> Option<TcpStream> {
        let address = format!("{}:{}", host, port);
        match TcpStream::connect(&address) {
            Ok(stream) => {
                stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
                stream
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .ok()?;
                println!("Connected to {}", address);
                Some(stream)
            }
            Err(e) => {
                warn!("Failed to connect: {}", e);
                None
            }
        }
    }

    /// Sends a minimal HTTP/1.1 `GET` request and reads the response **until
    /// the server closes the connection**.
    ///
    /// `Connection: close` and `Accept-Encoding: identity` are automatically
    /// sent.  The returned buffer contains the **raw** HTTP response (status
    /// line + headers + body).
    ///
    /// # Example
    ///
    /// ```
    /// let mut stream = Proxy::connect_to_server("example.com", 80)?;
    /// let raw = Proxy::send_http_request(&mut stream, "/index.html", "example.com")?;
    /// let (body, mime) = Proxy::parse_http_response_bytes(&raw);
    /// ```
    pub(crate) fn send_http_request(
        stream: &mut TcpStream,
        path: &str,
        host: &str,
    ) -> Option<Vec<u8>> {
        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n",
            path, host
        );
        stream.write_all(request.as_bytes()).ok()?;

        let mut buffer = Vec::new();
        let mut temp = [0u8; 8192];

        loop {
            match stream.read(&mut temp) {
                Ok(0) => break,
                Ok(n) => buffer.extend_from_slice(&temp[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    warn!("Failed to read from socket: {}", e);
                    break;
                }
            }
        }

        if buffer.is_empty() {
            None
        } else {
            Some(buffer)
        }
    }

    /// Upgrades the TCP stream with rustls, then performs the same logic as
    /// [`send_http_request`](Self::send_http_request).
    ///
    /// Server certificate validation uses the native root store (loaded once
    /// via `OnceLock`).  ALPN, SNI, and TLS 1.3 are handled automatically.
    pub(crate) fn send_https_request(
        stream: &mut TcpStream,
        path: &str,
        host: &str,
    ) -> Option<Vec<u8>> {
        static TLS_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

        let config = TLS_CONFIG.get_or_init(|| {
            let mut root_store = RootCertStore::empty();
            let certs = load_native_certs();
            for cert in certs.certs {
                let _ = root_store.add(cert);
            }

            Arc::new(
                ClientConfig::builder()
                    .with_root_certificates(root_store)
                    .with_no_client_auth(),
            )
        });

        let server_name = match ServerName::try_from(host.to_string()) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let conn = match ClientConnection::new(config.clone(), server_name) {
            Ok(c) => c,
            Err(_) => return None,
        };

        let mut tls_stream = StreamOwned::new(conn, stream);

        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            path, host
        );

        if tls_stream.write_all(request.as_bytes()).is_err() {
            return None;
        }

        let mut response = Vec::new();
        if tls_stream.read_to_end(&mut response).is_err() {
            return None;
        }

        Some(response)
    }

    /// Minimal HTTP response parser.
    ///
    /// Returns `(body_bytes, content_type_string)`:
    /// - `Content-Length` and `Transfer-Encoding: chunked` are recognised
    /// - Headers are **not** exposed; only the body and the `Content-Type`
    ///   value are returned
    /// - If the response is malformed, the whole input is returned as the body
    ///   and `text/html` is assumed
    ///
    /// # Example
    ///
    /// ```
    /// let raw = Proxy::send_https_request(&mut tls_stream, "/api", "api.example.com")?;
    /// let (json, _mime) = Proxy::parse_http_response_bytes(&raw);
    /// ```
    pub(crate) fn parse_http_response_bytes(response: &[u8]) -> (Vec<u8>, String) {
        if let Some(header_end) = find_header_end(response) {
            let headers_str = String::from_utf8_lossy(&response[..header_end]);
            let mut content_type = "text/html".to_string();
            let mut is_chunked = false;
            let mut content_length = None;

            for line in headers_str.lines() {
                let lower = line.to_lowercase();
                if lower.starts_with("content-type:") {
                    content_type = line
                        .split(':')
                        .nth(1)
                        .unwrap_or("text/html")
                        .trim()
                        .to_string();
                }
                if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
                    is_chunked = true;
                }
                if lower.starts_with("content-length:") {
                    content_length = line
                        .split(':')
                        .nth(1)
                        .and_then(|v| v.trim().parse::<usize>().ok());
                }
            }

            let raw_body = &response[header_end + 4..];
            let body = if is_chunked {
                decode_chunked_body(raw_body)
            } else if let Some(len) = content_length {
                raw_body[..std::cmp::min(len, raw_body.len())].to_vec()
            } else {
                raw_body.to_vec()
            };

            (body, content_type)
        } else {
            (response.to_vec(), "text/html".to_string())
        }
    }
}

/// Returns the index of the first `\r\n\r\n` sequence, marking the end of
/// HTTP headers.
pub(crate) fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Decodes a **chunked** HTTP body (RFC 9112 §7.1).
///
/// Stops at the final zero-length chunk; trailers are ignored.
fn decode_chunked_body(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // find line end (\r\n)
        let line_end = match data[pos..].iter().position(|&b| b == b'\n') {
            Some(end) => pos + end + 1,
            None => break,
        };

        let line_text = String::from_utf8_lossy(&data[pos..line_end]);
        let size_line = line_text.trim().split(';').next().unwrap();
        let size = match usize::from_str_radix(size_line, 16) {
            Ok(s) => s,
            Err(_) => break,
        };

        if size == 0 {
            break;
        }

        pos = line_end;
        if pos + size > data.len() {
            break;
        }

        result.extend_from_slice(&data[pos..pos + size]);
        pos += size;

        // skip \r\n
        if pos + 2 <= data.len() && &data[pos..pos + 2] == b"\r\n" {
            pos += 2;
        }
    }

    result
}
