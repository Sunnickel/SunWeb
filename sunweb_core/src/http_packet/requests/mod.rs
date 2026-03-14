//! HTTP request parsing and convenient parameter accessors.
//!
//! This module exposes the [`HTTPRequest`] type, which turns a raw `&[u8]` into
//! a strongly-typed value with helpers for headers, query strings, path
//! parameters, url-encoded forms, JSON bodies, and cookies.
use crate::http_packet::body::Body;
use crate::http_packet::header::content_types::ContentType;
use crate::http_packet::header::headers::cookie::Cookie;
use crate::http_packet::header::http_method::HTTPMethod;
use crate::http_packet::header::HTTPHeader;
use crate::http_packet::HTTPMessage;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct HTTPRequest {
    pub(crate) method: HTTPMethod,
    pub(crate) path: String,
    pub(crate) message: HTTPMessage,
    pub(crate) query_params: HashMap<String, String>,
    pub(crate) path_params: HashMap<String, String>,
    pub(crate) form_params: HashMap<String, String>,
    pub(crate) cookie_jar: Vec<Cookie>,
}

impl HTTPRequest {
    /// Parses a complete HTTP/1.1 request from raw bytes.
    ///
    /// Returns `Err(description)` on any protocol violation or unsupported
    /// encoding.  On success, query parameters, cookies and (when applicable)
    /// form parameters are already parsed and ready to use.
    ///
    /// # Example
    ///
    /// ```
    /// let raw = b"GET /search?q=rust HTTP/1.1\r\nHost: example.com\r\n\r\n";
    /// let req = HTTPRequest::parse(raw).unwrap();
    /// assert_eq!(req.query_param("q"), Some("rust".into()));
    /// ```
    pub fn parse(raw_request: &[u8]) -> Result<Self, String> {
        let request_str = String::from_utf8(raw_request.to_vec())
            .map_err(|e| format!("Invalid UTF-8 in request: {}", e))?;

        if request_str.trim().is_empty() {
            return Err("Empty request".into());
        }

        let mut lines = request_str.lines();
        let request_line = lines.next().ok_or("Empty request");
        let parts: Vec<&str>;

        match request_line {
            Ok(line) => {
                parts = line.split_whitespace().collect();
                if parts.len() != 3 {
                    return Err("Invalid request line format".to_string());
                }
            }
            Err(_) => {
                return Err("Invalid request line format".to_string());
            }
        }

        let method = HTTPMethod::from_str(parts[0])
            .map_err(|_| format!("Unknown HTTP method: {}", parts[0]))?;
        let path = parts[1].to_string();
        let http_version = parts[2].to_string();

        let mut header_map = HashMap::new();

        for line in &mut lines {
            if line.is_empty() {
                break;
            }
            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                header_map.insert(name, value);
            }
        }

        let headers = HTTPHeader::new(header_map);

        // Parse body if Content-Length is present
        let body = if let Ok(Some(content_length_str)) = headers
            .get_header("Content-Length")
            .ok_or("No content length")
            .map(|h| Some(h))
        {
            if let Ok(content_length) = usize::from_str(&content_length_str) {
                let remaining = request_str
                    .lines()
                    .last()
                    .map(|l| l.as_bytes())
                    .unwrap_or(&[]);
                if remaining.len() >= content_length {
                    Some(remaining[..content_length].to_vec())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let message = HTTPMessage {
            http_version,
            headers,
            body,
        };

        let mut request = Self {
            method,
            path,
            message,
            query_params: HashMap::new(),
            path_params: HashMap::new(),
            form_params: HashMap::new(),
            cookie_jar: Vec::new(),
        };

        request.parse_query_params();
        request.parse_cookies();
        request.parse_form_params();

        Ok(request)
    }

    // ── Method ───────────────────────────────────────────────────────────────

    /// The HTTP method
    /// ```rust
    /// req.method() // => HTTPMethod::GET
    /// ```
    pub fn method(&self) -> &HTTPMethod {
        &self.method
    }

    pub fn is_get(&self) -> bool { self.method == HTTPMethod::GET }
    pub fn is_post(&self) -> bool { self.method == HTTPMethod::POST }
    pub fn is_put(&self) -> bool { self.method == HTTPMethod::PUT }
    pub fn is_delete(&self) -> bool { self.method == HTTPMethod::DELETE }
    pub fn is_patch(&self) -> bool { self.method == HTTPMethod::PATCH }

    // ── Path ─────────────────────────────────────────────────────────────────

    /// Path without query string — "/users/123"
    pub fn path(&self) -> &str {
        self.path.split('?').next().unwrap_or(&self.path)
    }

    /// Full path including query string — "/users/123?foo=bar"
    pub fn full_path(&self) -> &str {
        &self.path
    }

    // ── Headers ──────────────────────────────────────────────────────────────

    /// Case-insensitive header lookup
    /// ```rust
    /// req.header("content-type")
    /// req.header("Content-Type") // same result
    /// ```
    pub fn header(&self, name: &str) -> Option<String> {
        self.message.headers.get_header(name)
    }

    pub fn has_header(&self, name: &str) -> bool {
        self.message.headers.get_header(name).is_some()
    }

    pub fn content_type(&self) -> Option<ContentType> {
        self.header("Content-Type")
            .and_then(|v| ContentType::from_str(&v).ok())
    }

    pub fn host(&self) -> Option<String> { self.header("Host") }
    pub fn user_agent(&self) -> Option<String> { self.header("User-Agent") }
    pub fn authorization(&self) -> Option<String> { self.header("Authorization") }

    /// Extracts token from "Authorization: Bearer <token>"
    /// ```rust
    /// let token = req.bearer_token()?;
    /// ```
    pub fn bearer_token(&self) -> Option<String> {
        self.authorization()
            .and_then(|a| a.strip_prefix("Bearer ").map(|t| t.to_string()))
    }

    /// True if client accepts JSON responses
    pub fn accepts_json(&self) -> bool {
        self.header("Accept")
            .map(|a| a.contains("application/json"))
            .unwrap_or(false)
    }

    pub fn is_json(&self) -> bool {
        self.header("Content-Type")
            .map(|c| c.contains("application/json"))
            .unwrap_or(false)
    }

    pub fn is_form(&self) -> bool {
        self.header("Content-Type")
            .map(|c| c.contains("application/x-www-form-urlencoded"))
            .unwrap_or(false)
    }

    // ── Body ─────────────────────────────────────────────────────────────────

    /// Returns the body — use .as_string(), .as_json(), .as_form() on it
    /// ```rust
    /// req.body()?.as_string()
    /// req.body()?.as_json::<MyStruct>()
    /// req.body()?.as_form()
    /// req.body()?.len()
    /// ```
    pub fn body(&self) -> Option<Body> {
        self.message.body.as_ref().map(|b| Body::new(b.clone()))
    }

    pub fn has_body(&self) -> bool {
        self.message.body.as_ref().map(|b| !b.is_empty()).unwrap_or(false)
    }

    // ── Query Params ─────────────────────────────────────────────────────────

    /// Get a query param by key
    /// ```rust
    /// // /search?q=rust&page=2
    /// req.query("q")        // => Some("rust")
    /// req.query("missing")  // => None
    /// ```
    pub fn query(&self, key: &str) -> Option<&str> {
        self.query_params.get(key).map(|s| s.as_str())
    }

    /// Get a query param parsed into any type
    /// ```rust
    /// let page: u32 = req.query_as("page").unwrap_or(1);
    /// let enabled: bool = req.query_as("enabled").unwrap_or(false);
    /// ```
    pub fn query_as<T: FromStr>(&self, key: &str) -> Option<T> {
        self.query_params.get(key).and_then(|s| s.parse().ok())
    }

    /// Get a query param or a fallback value
    /// ```rust
    /// let sort = req.query_or("sort", "asc");
    /// ```
    pub fn query_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.query_params.get(key).map(|s| s.as_str()).unwrap_or(default)
    }

    pub fn all_query(&self) -> &HashMap<String, String> {
        &self.query_params
    }

    // ── Path Params ──────────────────────────────────────────────────────────

    /// Get a path segment by name
    /// ```rust
    /// // route: /users/:id  request: /users/42
    /// req.param("id")  // => Some("42")
    /// ```
    pub fn param(&self, key: &str) -> Option<&str> {
        self.path_params.get(key).map(|s| s.as_str())
    }

    /// Get a path param parsed into any type
    /// ```rust
    /// let id: u64 = req.param_as("id").unwrap_or(0);
    /// ```
    pub fn param_as<T: FromStr>(&self, key: &str) -> Option<T> {
        self.path_params.get(key).and_then(|s| s.parse().ok())
    }

    pub fn all_params(&self) -> &HashMap<String, String> {
        &self.path_params
    }

    /// Used by the router — not intended for direct use
    pub fn set_path_param(&mut self, key: String, value: String) {
        self.path_params.insert(key, value);
    }

    // ── Form Params ──────────────────────────────────────────────────────────

    /// Get a form field by key (urlencoded or JSON body)
    /// ```rust
    /// let name = req.form("username")?;
    /// ```
    pub fn form(&self, key: &str) -> Option<&str> {
        self.form_params.get(key).map(|s| s.as_str())
    }

    /// Get a form field parsed into any type
    /// ```rust
    /// let age: u32 = req.form_as("age").unwrap_or(0);
    /// ```
    pub fn form_as<T: FromStr>(&self, key: &str) -> Option<T> {
        self.form_params.get(key).and_then(|s| s.parse().ok())
    }

    pub fn all_form(&self) -> &HashMap<String, String> {
        &self.form_params
    }

    // ── Cookies ──────────────────────────────────────────────────────────────

    /// Get a cookie by name
    /// ```rust
    /// let session = req.cookie("session_id")?;
    /// println!("{}", session.value);
    /// ```
    pub fn cookie(&self, name: &str) -> Option<&Cookie> {
        self.cookie_jar.iter().find(|c| c.key == name)
    }

    pub fn has_cookie(&self, name: &str) -> bool {
        self.cookie(name).is_some()
    }

    pub fn all_cookies(&self) -> &[Cookie] {
        &self.cookie_jar
    }

    // ── Private ──────────────────────────────────────────────────────────────

    pub(crate) fn parse_query_params(&mut self) {
        if let Some(query_start) = self.path.find('?') {
            let query_string = &self.path[query_start + 1..].to_string();
            for pair in query_string.split('&') {
                if let Some(eq_pos) = pair.find('=') {
                    let key = url_decode(&pair[..eq_pos]);
                    let value = url_decode(&pair[eq_pos + 1..]);
                    self.query_params.insert(key, value);
                } else {
                    self.query_params.insert(url_decode(pair), String::new());
                }
            }
        }
    }

    pub(crate) fn parse_cookies(&mut self) {
        if let Some(cookie_header) = self.header("Cookie") {
            for cookie in cookie_header.split(';') {
                if let Some(eq_pos) = cookie.find('=') {
                    let key = cookie[..eq_pos].trim().to_string();
                    let value = cookie[eq_pos + 1..].trim().to_string();
                    let host = self.host().unwrap_or_default();
                    self.cookie_jar.push(Cookie::new(&key, &value, &host));
                }
            }
        }
    }

    pub(crate) fn parse_form_params(&mut self) {
        if let Some(body) = &self.message.body.clone() {
            if let Ok(body_str) = String::from_utf8(body.clone()) {
                let content_type = self.header("Content-Type").unwrap_or_default();
                if content_type.contains("application/x-www-form-urlencoded") {
                    for pair in body_str.split('&') {
                        if let Some(eq_pos) = pair.find('=') {
                            self.form_params.insert(url_decode(&pair[..eq_pos]), url_decode(&pair[eq_pos + 1..]));
                        } else {
                            self.form_params.insert(url_decode(pair), String::new());
                        }
                    }
                } else if content_type.contains("application/json") {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&body_str) {
                        if let Some(obj) = json_value.as_object() {
                            for (key, value) in obj {
                                let value_str = match value {
                                    serde_json::Value::String(s) => s.clone(),
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::Bool(b) => b.to_string(),
                                    serde_json::Value::Null => String::new(),
                                    _ => value.to_string(),
                                };
                                self.form_params.insert(key.clone(), value_str);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn url_decode(encoded: &str) -> String {
    let mut decoded = String::new();
    let mut chars = encoded.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '%' => {
                if let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
                    if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                        decoded.push(byte as char);
                    }
                }
            }
            '+' => decoded.push(' '),
            _ => decoded.push(ch),
        }
    }
    decoded
}