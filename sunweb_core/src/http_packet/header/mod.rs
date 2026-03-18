use crate::http_packet::header::connection::ConnectionType;
use crate::http_packet::header::content_types::ContentType;
use crate::http_packet::header::content_types::text::TextSubType;
use crate::http_packet::header::headers::cache_control::CacheControl;
use crate::http_packet::header::headers::content_encoding::ContentEncoding;
use crate::http_packet::header::headers::content_security_policy::{CspBuilder, CspDirective};
use crate::http_packet::header::headers::frame_option::FrameOption;
use crate::http_packet::header::headers::referer_policy::ReferrerPolicy;
use crate::http_packet::header::headers::transfer_encoding::TransferEncoding;
use chrono::{DateTime, Utc};
use headers::cookie::Cookie;
use std::collections::HashMap;

pub mod connection;
pub mod content_types;
pub mod headers;
pub mod http_method;

/// Manages the HTTP headers attached to a response.
///
/// Handles standard headers, cookies, security policies, CORS, and caching.
/// Constructed internally — access it through [`Response`].
#[derive(Clone, Debug)]
pub struct HTTPHeader {
    pub(crate) values: HashMap<String, String>,
    pub content_type: ContentType,
    pub content_length: Option<u64>,
    pub connection: ConnectionType,
    cookies: Vec<Cookie>,
}

impl HTTPHeader {
    /// Creates a new `HTTPHeader` with the given initial header map.
    pub(crate) fn new(values: HashMap<String, String>) -> Self {
        Self {
            values,
            content_type: ContentType::Text(TextSubType::Html),
            content_length: None,
            connection: ConnectionType::KeepAlive,
            cookies: Vec::new(),
        }
    }

    /// Serializes all headers and cookies into HTTP wire format.
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

    /// Inserts or overwrites a raw header key-value pair.
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
    }

    /// Returns the value of a header by name, case-insensitively.
    /// Returns `None` if the header is not present.
    pub(crate) fn get_header(&self, header: &str) -> Option<String> {
        let header_lower = header.to_lowercase();
        if let Some(value) = self.values.get(header) {
            return Some(value.clone());
        }
        for (k, v) in &self.values {
            if k.to_lowercase() == header_lower {
                return Some(v.clone());
            }
        }
        None
    }

    // ── Cookies ──────────────────────────────────────────────────────────────

    /// Adds a `Set-Cookie` header for the given cookie.
    #[allow(dead_code)]
    pub(crate) fn set_cookie(&mut self, cookie: Cookie) {
        self.cookies.push(cookie);
    }

    /// Expires a cookie by setting its expiry to the Unix epoch.
    #[allow(dead_code)]
    pub(crate) fn expire_cookie(&mut self, mut cookie: Cookie) {
        cookie = cookie.expires(Some(0));
        self.cookies.push(cookie);
    }

    // ── Standard headers ─────────────────────────────────────────────────────

    /// Sets the `Date` header to the current UTC time.
    pub(crate) fn set_date_now(&mut self) {
        let now = Utc::now();
        self.add_header("Date", &now.format("%a, %d %b %Y %H:%M:%S GMT").to_string());
    }

    /// Sets the `Server` header (e.g. `"SunWeb/0.3"`).
    pub(crate) fn set_server(&mut self, server_name: &str) {
        self.add_header("Server", server_name);
    }

    /// Sets the `Location` header for redirects.
    pub(crate) fn set_location(&mut self, url: &str) {
        self.add_header("Location", url);
    }

    /// Sets the `Content-Encoding` header from a [`ContentEncoding`] value.
    pub(crate) fn set_content_encoding(&mut self, encoding: ContentEncoding) {
        self.add_header("Content-Encoding", &encoding.as_str());
    }

    /// Sets the `Transfer-Encoding` header (e.g. `"chunked"`).
    #[allow(dead_code)]
    pub(crate) fn set_transfer_encoding(&mut self, encoding: TransferEncoding) {
        self.add_header("Transfer-Encoding", &encoding.as_str());
    }

    /// Sets the `ETag` header, wrapping the value in quotes.
    #[allow(dead_code)]
    pub(crate) fn set_etag(&mut self, etag: &str) {
        self.add_header("ETag", &format!("\"{}\"", etag));
    }

    /// Sets the `Last-Modified` header from a UTC timestamp.
    #[allow(dead_code)]
    pub(crate) fn set_last_modified(&mut self, datetime: DateTime<Utc>) {
        self.add_header(
            "Last-Modified",
            &datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
        );
    }

    // ── Cache headers ─────────────────────────────────────────────────────────

    /// Sets the `Cache-Control` header from a [`CacheControl`] directive.
    ///
    /// Use `CacheControl::Multiple` to combine several directives at once.
    ///
    /// # Example
    /// ```rust,ignore
    /// headers.set_cache_control(CacheControl::MaxAge(3600));
    /// headers.set_cache_control(CacheControl::Multiple(vec![
    ///     CacheControl::Public,
    ///     CacheControl::MaxAge(86400),
    /// ]));
    /// headers.set_cache_control(CacheControl::Multiple(vec![
    ///     CacheControl::NoCache,
    ///     CacheControl::NoStore,
    ///     CacheControl::MustRevalidate,
    /// ]));
    /// ```
    #[allow(dead_code)]
    pub(crate) fn set_cache_control(&mut self, directive: CacheControl) {
        self.add_header("Cache-Control", &directive.as_str());
    }

    // ── Security headers ──────────────────────────────────────────────────────

    /// Sets `X-Content-Type-Options: nosniff` to prevent MIME sniffing.
    pub(crate) fn set_nosniff(&mut self) {
        self.add_header("X-Content-Type-Options", "nosniff");
    }

    /// Sets `X-Frame-Options` to guard against clickjacking.
    pub(crate) fn set_frame_options(&mut self, option: FrameOption) {
        self.add_header("X-Frame-Options", option.as_str());
    }

    /// Sets `Strict-Transport-Security` to enforce HTTPS connections.
    ///
    /// Pass `include_subdomains: true` to also cover all subdomains.
    pub(crate) fn set_hsts(&mut self, max_age_seconds: u64, include_subdomains: bool) {
        let mut value = format!("max-age={}", max_age_seconds);
        if include_subdomains {
            value.push_str("; includeSubDomains");
        }
        self.add_header("Strict-Transport-Security", &value);
    }

    /// Sets `Content-Security-Policy` from a [`CspBuilder`].
    ///
    /// # Example
    /// ```rust,ignore
    /// let policy = CspBuilder::new()
    ///     .directive(CspDirective::DefaultSrc(vec!["'self'".to_string()]))
    ///     .directive(CspDirective::ScriptSrc(vec!["'self'".to_string(), "'unsafe-inline'".to_string()]))
    ///     .directive(CspDirective::ImgSrc(vec!["'self'".to_string(), "https:".to_string()]));
    ///
    /// headers.set_csp(policy);
    /// ```
    #[allow(dead_code)]
    pub(crate) fn set_csp(&mut self, policy: CspBuilder) {
        self.add_header("Content-Security-Policy", &policy.build());
    }

    /// Sets `X-XSS-Protection`. Prefer CSP in modern browsers — this is a
    /// legacy header included for compatibility.
    pub(crate) fn set_xss_protection(&mut self, enabled: bool) {
        let value = if enabled { "1; mode=block" } else { "0" };
        self.add_header("X-XSS-Protection", value);
    }

    #[allow(dead_code)]
    pub(crate) fn set_referrer_policy(&mut self, policy: ReferrerPolicy) {
        self.add_header("Referrer-Policy", policy.as_str());
    }

    /// Applies a sensible default set of security headers:
    /// `X-Content-Type-Options`, `X-Frame-Options: Deny`,
    /// `X-XSS-Protection`, and `Content-Security-Policy: default-src 'self'`.
    pub(crate) fn apply_security_headers(&mut self) {
        self.set_nosniff();
        self.set_frame_options(FrameOption::Deny);
        self.set_xss_protection(true);
        self.set_referrer_policy(ReferrerPolicy::StrictOriginWhenCrossOrigin);
        self.set_csp(
            CspBuilder::new().directive(CspDirective::DefaultSrc(vec!["'self'".to_string()])),
        );
    }

    // ── CORS headers ──────────────────────────────────────────────────────────

    /// Sets `Access-Control-Allow-Origin`. Use `"*"` to allow all origins.
    pub(crate) fn set_cors_origin(&mut self, origin: &str) {
        self.add_header("Access-Control-Allow-Origin", origin);
    }

    /// Sets `Access-Control-Allow-Methods` from a slice of method strings.
    pub(crate) fn set_cors_methods(&mut self, methods: &[&str]) {
        self.add_header("Access-Control-Allow-Methods", &methods.join(", "));
    }

    /// Sets `Access-Control-Allow-Headers` from a slice of header names.
    pub(crate) fn set_cors_headers(&mut self, headers: &[&str]) {
        self.add_header("Access-Control-Allow-Headers", &headers.join(", "));
    }

    /// Sets `Access-Control-Max-Age` — how long preflight results may be cached.
    pub(crate) fn set_cors_max_age(&mut self, seconds: u64) {
        self.add_header("Access-Control-Max-Age", &seconds.to_string());
    }

    /// Sets `Access-Control-Allow-Credentials: true` when `allow` is `true`.
    pub(crate) fn set_cors_credentials(&mut self, allow: bool) {
        if allow {
            self.add_header("Access-Control-Allow-Credentials", "true");
        }
    }

    /// Applies permissive CORS headers allowing all origins and methods.
    ///
    /// ⚠️ **Development only** — do not use in production.
    pub(crate) fn apply_cors_permissive(&mut self) {
        self.set_cors_origin("*");
        self.set_cors_methods(&["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"]);
        self.set_cors_headers(&["*"]);
        self.set_cors_max_age(86400);
    }
}
