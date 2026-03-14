use headers::cookie::Cookie;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::http_packet::header::connection::ConnectionType;
use crate::http_packet::header::content_types::ContentType;
use crate::http_packet::header::content_types::text::TextSubType;
use crate::http_packet::header::headers::frame_option::FrameOption;

pub mod connection;
pub mod content_types;
pub mod headers;
pub mod http_method;

/// Represents HTTP response headers
///
/// The `HTTPHeader` struct manages HTTP headers including content type,
/// cookies, security headers, and standard HTTP headers.
#[derive(Clone, Debug)]
pub struct HTTPHeader {
    pub(crate) values: HashMap<String, String>,
    pub content_type: ContentType,
    pub content_length: Option<u64>,
    pub connection: ConnectionType,
    cookies: Vec<Cookie>,
}

impl HTTPHeader {
    /// Creates new response headers
    pub(crate) fn new(values: HashMap<String, String>) -> Self {
        Self {
            values,
            content_type: ContentType::Text(TextSubType::Html),
            content_length: None,
            connection: ConnectionType::KeepAlive,
            cookies: Vec::new(),
        }
    }

    /// Converts the headers to a string representation
    pub(crate) fn as_str(&self) -> String {
        let mut result = String::new();

        for (k, v) in &self.values {
            result.push_str(&format!("{}: {}\r\n", k, v));
        }

        for cookie in &self.cookies {
            result.push_str(&format!("Set-Cookie: {}\r\n", cookie.as_string()));
        }

        result
    }

    /// Adds a header to the response
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
    }

    /// Gets a header value by name (case-insensitive)
    pub(crate) fn get_header(&self, header: &str) -> Option<String> {
        let header_lower = header.to_lowercase();

        // Try exact match first
        if let Some(value) = self.values.get(header) {
            return Some(value.clone());
        }

        // Try case-insensitive match
        for (k, v) in &self.values {
            if k.to_lowercase() == header_lower {
                return Some(v.clone());
            }
        }

        None
    }

    /// Sets a cookie in the response headers
    ///
    /// # Arguments
    /// * `cookie` - The cookie to set
    ///
    /// # Examples
    /// ```
    /// use your_crate::{HTTPHeader, Cookie};
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// let cookie = Cookie::new("session", "abc123");
    /// headers.set_cookie(cookie);
    /// ```
    pub(crate) fn set_cookie(&mut self, cookie: Cookie) {
        self.cookies.push(cookie);
    }

    /// Expires a cookie by setting its expiration to the Unix epoch
    ///
    /// # Arguments
    /// * `cookie` - The cookie to expire
    ///
    /// # Examples
    /// ```
    /// use your_crate::{HTTPHeader, Cookie};
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// let cookie = Cookie::new("session", "abc123");
    /// headers.expire_cookie(cookie);
    /// ```
    pub(crate) fn expire_cookie(&mut self, mut cookie: Cookie) {
        cookie = cookie.expires(Some(0));
        self.cookies.push(cookie);
    }

    /// Sets the Date header to the current UTC time
    ///
    /// The Date header represents the date and time at which the message was originated.
    /// Format: `Day, DD Mon YYYY HH:MM:SS GMT`
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_date_now();
    /// ```
    pub(crate) fn set_date_now(&mut self) {
        let now = Utc::now();
        self.add_header("Date", &now.format("%a, %d %b %Y %H:%M:%S GMT").to_string());
    }

    /// Sets the Server header to identify the server software
    ///
    /// # Arguments
    /// * `server_name` - The server identifier (e.g., "MyServer/1.0")
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_server("MyWebServer/1.0");
    /// ```
    pub(crate) fn set_server(&mut self, server_name: &str) {
        self.add_header("Server", server_name);
    }

    /// Sets the Location header for HTTP redirects
    ///
    /// # Arguments
    /// * `url` - The URL to redirect to
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_location("https://example.com/new-page");
    /// ```
    pub(crate) fn set_location(&mut self, url: &str) {
        self.add_header("Location", url);
    }

    /// Sets the Cache-Control header with custom directives
    ///
    /// # Arguments
    /// * `directive` - Cache control directive string (e.g., "max-age=3600")
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_cache_control("max-age=3600, public");
    /// ```
    pub(crate) fn set_cache_control(&mut self, directive: &str) {
        self.add_header("Cache-Control", directive);
    }

    /// Sets headers to completely disable caching
    ///
    /// Sets Cache-Control, Pragma, and Expires headers to prevent any caching.
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_no_cache();
    /// ```
    pub(crate) fn set_no_cache(&mut self) {
        self.add_header("Cache-Control", "no-cache, no-store, must-revalidate");
        self.add_header("Pragma", "no-cache");
        self.add_header("Expires", "0");
    }

    /// Sets the Cache-Control max-age directive
    ///
    /// # Arguments
    /// * `seconds` - Number of seconds the resource should be cached
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_max_age(3600); // Cache for 1 hour
    /// ```
    pub(crate) fn set_max_age(&mut self, seconds: u64) {
        self.add_header("Cache-Control", &format!("max-age={}", seconds));
    }

    /// Sets the ETag header for cache validation
    ///
    /// The ETag is a unique identifier for a specific version of a resource.
    ///
    /// # Arguments
    /// * `etag` - The entity tag value (will be wrapped in quotes)
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_etag("33a64df551425fcc55e4d42a148795d9f25f89d4");
    /// ```
    pub(crate) fn set_etag(&mut self, etag: &str) {
        self.add_header("ETag", &format!("\"{}\"", etag));
    }

    /// Sets the Last-Modified header
    ///
    /// Indicates the last time the resource was modified.
    ///
    /// # Arguments
    /// * `datetime` - The modification timestamp
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    /// use chrono::Utc;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_last_modified(Utc::now());
    /// ```
    pub(crate) fn set_last_modified(&mut self, datetime: DateTime<Utc>) {
        self.add_header(
            "Last-Modified",
            &datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
        );
    }

    /// Sets the Content-Encoding header
    ///
    /// Indicates what encodings have been applied to the response body.
    ///
    /// # Arguments
    /// * `encoding` - The encoding type (e.g., "gzip", "deflate", "br")
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_content_encoding("gzip");
    /// ```
    pub(crate) fn set_content_encoding(&mut self, encoding: &str) {
        self.add_header("Content-Encoding", encoding);
    }

    /// Sets the Transfer-Encoding header
    ///
    /// Specifies the form of encoding used to safely transfer the payload.
    ///
    /// # Arguments
    /// * `encoding` - The transfer encoding (e.g., "chunked")
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_transfer_encoding("chunked");
    /// ```
    pub(crate) fn set_transfer_encoding(&mut self, encoding: &str) {
        self.add_header("Transfer-Encoding", encoding);
    }

    // ===== Security Headers =====

    /// Sets X-Content-Type-Options: nosniff
    ///
    /// Prevents browsers from MIME-sniffing a response away from the declared content-type.
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_nosniff();
    /// ```
    pub(crate) fn set_nosniff(&mut self) {
        self.add_header("X-Content-Type-Options", "nosniff");
    }

    /// Sets X-Frame-Options to prevent clickjacking attacks
    ///
    /// # Arguments
    /// * `option` - The frame option policy
    ///
    /// # Examples
    /// ```
    /// use your_crate::{HTTPHeader, FrameOption};
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_frame_options(FrameOption::Deny);
    /// ```
    pub(crate) fn set_frame_options(&mut self, option: FrameOption) {
        self.add_header("X-Frame-Options", option.as_str());
    }

    /// Sets Strict-Transport-Security (HSTS) header
    ///
    /// Forces clients to use HTTPS for future requests.
    ///
    /// # Arguments
    /// * `max_age_seconds` - How long (in seconds) browsers should remember to use HTTPS
    /// * `include_subdomains` - Whether to apply HSTS to all subdomains
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_hsts(31536000, true); // 1 year, include subdomains
    /// ```
    pub(crate) fn set_hsts(&mut self, max_age_seconds: u64, include_subdomains: bool) {
        let mut value = format!("max-age={}", max_age_seconds);
        if include_subdomains {
            value.push_str("; includeSubDomains");
        }
        self.add_header("Strict-Transport-Security", &value);
    }

    /// Sets Content-Security-Policy header
    ///
    /// Helps prevent XSS, clickjacking, and other code injection attacks.
    ///
    /// # Arguments
    /// * `policy` - The CSP policy string
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_csp("default-src 'self'; script-src 'self' 'unsafe-inline'");
    /// ```
    pub(crate) fn set_csp(&mut self, policy: &str) {
        self.add_header("Content-Security-Policy", policy);
    }

    /// Sets X-XSS-Protection header
    ///
    /// Legacy header that enables browser's XSS filtering. Modern browsers prefer CSP.
    ///
    /// # Arguments
    /// * `enabled` - Whether to enable XSS protection
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_xss_protection(true);
    /// ```
    pub(crate) fn set_xss_protection(&mut self, enabled: bool) {
        let value = if enabled { "1; mode=block" } else { "0" };
        self.add_header("X-XSS-Protection", value);
    }

    /// Applies a set of common security headers
    ///
    /// Sets: X-Content-Type-Options, X-Frame-Options, X-XSS-Protection, and a basic CSP.
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.apply_security_headers();
    /// ```
    pub(crate) fn apply_security_headers(&mut self) {
        self.set_nosniff();
        self.set_frame_options(FrameOption::Deny);
        self.set_xss_protection(true);
        self.set_csp("default-src 'self'");
    }

    // ===== CORS Headers =====

    /// Sets Access-Control-Allow-Origin header
    ///
    /// Specifies which origins can access the resource.
    ///
    /// # Arguments
    /// * `origin` - The allowed origin (use "*" for all origins)
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_cors_origin("https://example.com");
    /// ```
    pub(crate) fn set_cors_origin(&mut self, origin: &str) {
        self.add_header("Access-Control-Allow-Origin", origin);
    }

    /// Sets Access-Control-Allow-Methods header
    ///
    /// Specifies which HTTP methods are allowed for cross-origin requests.
    ///
    /// # Arguments
    /// * `methods` - Array of allowed HTTP methods
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_cors_methods(&["GET", "POST", "PUT"]);
    /// ```
    pub(crate) fn set_cors_methods(&mut self, methods: &[&str]) {
        self.add_header("Access-Control-Allow-Methods", &methods.join(", "));
    }

    /// Sets Access-Control-Allow-Headers header
    ///
    /// Specifies which headers can be used in the actual request.
    ///
    /// # Arguments
    /// * `headers` - Array of allowed header names
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_cors_headers(&["Content-Type", "Authorization"]);
    /// ```
    pub(crate) fn set_cors_headers(&mut self, headers: &[&str]) {
        self.add_header("Access-Control-Allow-Headers", &headers.join(", "));
    }

    /// Sets Access-Control-Max-Age header
    ///
    /// Indicates how long preflight request results can be cached.
    ///
    /// # Arguments
    /// * `seconds` - Cache duration in seconds
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_cors_max_age(86400); // 24 hours
    /// ```
    pub(crate) fn set_cors_max_age(&mut self, seconds: u64) {
        self.add_header("Access-Control-Max-Age", &seconds.to_string());
    }

    /// Sets Access-Control-Allow-Credentials header
    ///
    /// Indicates whether the response can be exposed when credentials are included.
    ///
    /// # Arguments
    /// * `allow` - Whether to allow credentials
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.set_cors_credentials(true);
    /// ```
    pub(crate) fn set_cors_credentials(&mut self, allow: bool) {
        if allow {
            self.add_header("Access-Control-Allow-Credentials", "true");
        }
    }

    /// Applies permissive CORS headers allowing all origins and methods
    ///
    /// ⚠️ **Warning**: This is insecure for production use. Only use in development.
    ///
    /// # Examples
    /// ```
    /// use your_crate::HTTPHeader;
    /// use std::collections::HashMap;
    ///
    /// let mut headers = HTTPHeader::new(HashMap::new());
    /// headers.apply_cors_permissive(); // Only for development!
    /// ```
    pub(crate) fn apply_cors_permissive(&mut self) {
        self.set_cors_origin("*");
        self.set_cors_methods(&["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"]);
        self.set_cors_headers(&["*"]);
        self.set_cors_max_age(86400);
    }
}
