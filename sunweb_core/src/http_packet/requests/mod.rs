//! HTTP request parsing and convenient parameter accessors.
//!
//! This module exposes the [`HTTPRequest`] type, which turns a raw `&[u8]` into
//! a strongly-typed value with helpers for headers, query strings, path
//! parameters, url-encoded forms, JSON bodies, and cookies.

use crate::http_packet::HTTPMessage;
use crate::http_packet::body::Body;
use crate::http_packet::header::HTTPHeader;
use crate::http_packet::header::content_types::ContentType;
use crate::http_packet::header::headers::cookie::Cookie;
use crate::http_packet::header::http_method::HTTPMethod;
use std::collections::HashMap;
use std::str::FromStr;

/// A parsed HTTP/1.1 request.
///
/// Construct via [`HTTPRequest::parse`] — the framework does this for you
/// before passing the request to your route handler.
///
/// # Accessing request data
///
/// | What you need | Method |
/// |---|---|
/// | HTTP method | [`method()`](Self::method), [`is_get()`](Self::is_get), … |
/// | URL path | [`path()`](Self::path), [`full_path()`](Self::full_path) |
/// | Path param (`/users/:id`) | [`param()`](Self::param), [`param_as()`](Self::param_as) |
/// | Query string (`?key=val`) | [`query()`](Self::query), [`query_as()`](Self::query_as) |
/// | Request header | [`header()`](Self::header) |
/// | Body | [`body()`](Self::body) |
/// | Form field | [`form()`](Self::form), [`form_as()`](Self::form_as) |
/// | Cookie | [`cookie()`](Self::cookie) |
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
    /// Query parameters, cookies, and form parameters are parsed eagerly and
    /// available immediately after this call returns.
    ///
    /// # Errors
    /// Returns `Err(description)` on any protocol violation, unknown HTTP
    /// method, or invalid UTF-8.
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

        let body = if let Ok(Some(content_length_str)) = headers
            .get_header("Content-Length")
            .ok_or("No content length")
            .map(Some)
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

    /// Returns the HTTP method of the request.
    pub fn method(&self) -> &HTTPMethod {
        &self.method
    }

    /// Returns `true` if the method is `GET`.
    pub fn is_get(&self) -> bool {
        self.method == HTTPMethod::GET
    }

    /// Returns `true` if the method is `POST`.
    pub fn is_post(&self) -> bool {
        self.method == HTTPMethod::POST
    }

    /// Returns `true` if the method is `PUT`.
    pub fn is_put(&self) -> bool {
        self.method == HTTPMethod::PUT
    }

    /// Returns `true` if the method is `DELETE`.
    pub fn is_delete(&self) -> bool {
        self.method == HTTPMethod::DELETE
    }

    /// Returns `true` if the method is `PATCH`.
    pub fn is_patch(&self) -> bool {
        self.method == HTTPMethod::PATCH
    }

    // ── Path ─────────────────────────────────────────────────────────────────

    /// Returns the path without the query string (e.g. `"/users/42"`).
    pub fn path(&self) -> &str {
        self.path.split('?').next().unwrap_or(&self.path)
    }

    /// Returns the full path including any query string
    /// (e.g. `"/users/42?sort=asc"`).
    pub fn full_path(&self) -> &str {
        &self.path
    }

    // ── Headers ──────────────────────────────────────────────────────────────

    /// Returns the value of a header by name. Matching is case-insensitive.
    pub fn header(&self, name: &str) -> Option<String> {
        self.message.headers.get_header(name)
    }

    /// Returns `true` if the named header is present.
    pub fn has_header(&self, name: &str) -> bool {
        self.message.headers.get_header(name).is_some()
    }

    /// Returns the parsed `Content-Type`, or `None` if absent or unrecognised.
    pub fn content_type(&self) -> Option<ContentType> {
        self.header("Content-Type")
            .and_then(|v| ContentType::from_str(&v).ok())
    }

    /// Returns the `Host` header value.
    pub fn host(&self) -> Option<String> {
        self.header("Host")
    }

    /// Returns the `User-Agent` header value.
    pub fn user_agent(&self) -> Option<String> {
        self.header("User-Agent")
    }

    /// Returns the raw `Authorization` header value.
    pub fn authorization(&self) -> Option<String> {
        self.header("Authorization")
    }

    /// Extracts the bearer token from `Authorization: Bearer <token>`,
    /// returning `None` if the header is absent or not a bearer token.
    pub fn bearer_token(&self) -> Option<String> {
        self.authorization()
            .and_then(|a| a.strip_prefix("Bearer ").map(|t| t.to_string()))
    }

    /// Returns `true` if the `Accept` header includes `application/json`.
    pub fn accepts_json(&self) -> bool {
        self.header("Accept")
            .map(|a| a.contains("application/json"))
            .unwrap_or(false)
    }

    /// Returns `true` if `Content-Type` is `application/json`.
    pub fn is_json(&self) -> bool {
        self.header("Content-Type")
            .map(|c| c.contains("application/json"))
            .unwrap_or(false)
    }

    /// Returns `true` if `Content-Type` is `application/x-www-form-urlencoded`.
    pub fn is_form(&self) -> bool {
        self.header("Content-Type")
            .map(|c| c.contains("application/x-www-form-urlencoded"))
            .unwrap_or(false)
    }

    // ── Body ─────────────────────────────────────────────────────────────────

    /// Returns the request body, or `None` if there is no body.
    ///
    /// Call methods on [`Body`] to access the content:
    /// - [`Body::as_string`] — raw text
    /// - [`Body::as_json`] — deserialize via `serde_json`
    /// - [`Body::len`] — byte length
    pub fn body(&self) -> Option<Body> {
        self.message.body.as_ref().map(|b| Body::new(b.clone()))
    }

    /// Returns `true` if a non-empty body is present.
    pub fn has_body(&self) -> bool {
        self.message
            .body
            .as_ref()
            .map(|b| !b.is_empty())
            .unwrap_or(false)
    }

    // ── Query params ─────────────────────────────────────────────────────────

    /// Returns a query parameter by key, or `None` if not present.
    ///
    /// ```rust,ignore
    /// // GET /search?q=rust&page=2
    /// req.query("q")       // => Some("rust")
    /// req.query("missing") // => None
    /// ```
    pub fn query(&self, key: &str) -> Option<&str> {
        self.query_params.get(key).map(|s| s.as_str())
    }

    /// Returns a query parameter parsed into `T`, or `None` if absent or
    /// unparseable.
    ///
    /// ```rust,ignore
    /// let page: u32 = req.query_as("page").unwrap_or(1);
    /// ```
    pub fn query_as<T: FromStr>(&self, key: &str) -> Option<T> {
        self.query_params.get(key).and_then(|s| s.parse().ok())
    }

    /// Returns a query parameter or `default` if the key is absent.
    pub fn query_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.query_params
            .get(key)
            .map(|s| s.as_str())
            .unwrap_or(default)
    }

    /// Returns all query parameters as a map.
    pub fn all_query(&self) -> &HashMap<String, String> {
        &self.query_params
    }

    // ── Path params ──────────────────────────────────────────────────────────

    /// Returns a path parameter by name, or `None` if not present.
    ///
    /// ```rust,ignore
    /// // route: /users/:id   request: /users/42
    /// req.param("id") // => Some("42")
    /// ```
    pub fn param(&self, key: &str) -> Option<&str> {
        self.path_params.get(key).map(|s| s.as_str())
    }

    /// Returns a path parameter parsed into `T`, or `None` if absent or
    /// unparseable.
    ///
    /// ```rust,ignore
    /// let id: u64 = req.param_as("id").unwrap_or(0);
    /// ```
    pub fn param_as<T: FromStr>(&self, key: &str) -> Option<T> {
        self.path_params.get(key).and_then(|s| s.parse().ok())
    }

    /// Returns all path parameters as a map.
    pub fn all_params(&self) -> &HashMap<String, String> {
        &self.path_params
    }

    /// Inserts a path parameter. Called by the router — not intended for
    /// direct use in handlers.
    pub fn set_path_param(&mut self, key: String, value: String) {
        self.path_params.insert(key, value);
    }

    // ── Form params ──────────────────────────────────────────────────────────

    /// Returns a form field by key from a `application/x-www-form-urlencoded`
    /// or `application/json` body, or `None` if not present.
    ///
    /// ```rust,ignore
    /// let name = req.form("username")?;
    /// ```
    pub fn form(&self, key: &str) -> Option<&str> {
        self.form_params.get(key).map(|s| s.as_str())
    }

    /// Returns a form field parsed into `T`, or `None` if absent or
    /// unparseable.
    ///
    /// ```rust,ignore
    /// let age: u32 = req.form_as("age").unwrap_or(0);
    /// ```
    pub fn form_as<T: FromStr>(&self, key: &str) -> Option<T> {
        self.form_params.get(key).and_then(|s| s.parse().ok())
    }

    /// Returns all form parameters as a map.
    pub fn all_form(&self) -> &HashMap<String, String> {
        &self.form_params
    }

    // ── Cookies ──────────────────────────────────────────────────────────────

    /// Returns a cookie by name, or `None` if not present.
    ///
    /// ```rust,ignore
    /// let session = req.cookie("session_id")?;
    /// println!("{}", session.value);
    /// ```
    pub fn cookie(&self, name: &str) -> Option<&Cookie> {
        self.cookie_jar.iter().find(|c| c.key == name)
    }

    /// Returns `true` if a cookie with the given name is present.
    pub fn has_cookie(&self, name: &str) -> bool {
        self.cookie(name).is_some()
    }

    /// Returns all cookies sent with the request.
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
        let Some(body_str) = self
            .message
            .body
            .as_ref()
            .and_then(|b| String::from_utf8(b.clone()).ok())
        else {
            return;
        };

        let content_type = self.header("Content-Type").unwrap_or_default();

        if content_type.contains("application/x-www-form-urlencoded") {
            for pair in body_str.split('&') {
                if let Some(eq_pos) = pair.find('=') {
                    self.form_params
                        .insert(url_decode(&pair[..eq_pos]), url_decode(&pair[eq_pos + 1..]));
                } else {
                    self.form_params.insert(url_decode(pair), String::new());
                }
            }
        } else if content_type.contains("application/json") {
            let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&body_str) else {
                return;
            };
            let Some(obj) = json_value.as_object() else {
                return;
            };
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

/// Decodes a percent-encoded URL string, converting `%XX` sequences to their
/// corresponding characters and `+` to spaces.
pub(crate) fn url_decode(encoded: &str) -> String {
    let mut decoded = String::new();
    let mut chars = encoded.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '%' => {
                if let (Some(h1), Some(h2)) = (chars.next(), chars.next())
                    && let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16)
                {
                    decoded.push(byte as char);
                }
            }
            '+' => decoded.push(' '),
            _ => decoded.push(ch),
        }
    }
    decoded
}
