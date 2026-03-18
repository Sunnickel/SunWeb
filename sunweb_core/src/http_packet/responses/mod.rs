use crate::http_packet::HTTPMessage;
use crate::http_packet::body::Body;
use crate::http_packet::header::HTTPHeader;
use crate::http_packet::header::content_types::ContentType;
use crate::http_packet::header::content_types::application::ApplicationSubType;
use crate::http_packet::header::content_types::audio::AudioSubType;
use crate::http_packet::header::content_types::image::ImageSubType;
use crate::http_packet::header::content_types::text::TextSubType;
use crate::http_packet::header::content_types::video::VideoSubType;
use crate::http_packet::header::headers::cache_control::CacheControl;
use crate::http_packet::header::headers::content_encoding::ContentEncoding;
use crate::http_packet::header::headers::content_security_policy::CspBuilder;
use crate::http_packet::header::headers::frame_option::FrameOption;
use crate::http_packet::header::headers::referer_policy::ReferrerPolicy;
use crate::http_packet::header::headers::transfer_encoding::TransferEncoding;
use crate::http_packet::responses::status_code::StatusCode;
use std::collections::HashMap;

pub mod response_types;
pub mod status_code;

/// An HTTP response, combining a [`StatusCode`] with headers and an optional body.
///
/// Construct one via the shorthand constructors ([`Response::ok`],
/// [`Response::not_found`], etc.) or [`Response::new`] for full control.
/// Then set headers and a body before the framework serializes it with
/// [`to_bytes`](Response::to_bytes).
///
/// # Example
/// ```rust,ignore
/// let mut res = Response::ok();
/// res.set_html();
/// res.set_body_string("<h1>Hello</h1>".into());
/// ```
#[derive(Clone, Debug)]
pub struct Response {
    /// The status code sent in the response's first line.
    pub status_code: StatusCode,
    pub(crate) message: HTTPMessage,
}

/// Allows any type that can produce a [`Response`] to be returned from a route handler.
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

// ── Constructors ──────────────────────────────────────────────────────────────

impl Response {
    /// Creates a response with the given status code, no headers, and no body.
    pub fn new(status_code: StatusCode) -> Self {
        let headers = HTTPHeader::new(HashMap::new());
        let message = HTTPMessage::new("HTTP/1.1".to_string(), headers);
        Self {
            status_code,
            message,
        }
    }

    /// `200 OK` with no body.
    pub fn ok() -> Self {
        Self::new(StatusCode::Ok)
    }

    /// `404 Not Found` with no body.
    pub fn not_found() -> Self {
        Self::new(StatusCode::NotFound)
    }

    /// `500 Internal Server Error` with no body.
    pub fn internal_error() -> Self {
        Self::new(StatusCode::InternalServerError)
    }

    /// `405 Method Not Allowed` with no body.
    pub(crate) fn method_not_allowed() -> Self {
        Self::new(StatusCode::MethodNotAllowed)
    }

    /// `502 Bad Gateway` with no body.
    pub(crate) fn bad_gateway() -> Self {
        Self::new(StatusCode::BadGateway)
    }

    /// Redirect response with a `Location` header.
    ///
    /// Uses `308 Permanent Redirect` or `307 Temporary Redirect` depending
    /// on the `permanent` flag.
    pub fn redirect(location: &str, permanent: bool) -> Self {
        let status = if permanent {
            StatusCode::PermanentRedirect
        } else {
            StatusCode::TemporaryRedirect
        };
        let mut response = Self::new(status);
        response.set_location(location);
        response
    }
}

// ── Headers & body ────────────────────────────────────────────────────────────

impl Response {
    /// Inserts or overwrites a raw header key-value pair.
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.message.headers.add_header(key, value);
    }

    /// Returns the value of a header by name (case-insensitive), or `None`.
    pub fn get_header(&self, name: &str) -> Option<String> {
        self.message.headers.get_header(name)
    }

    /// Returns mutable access to the underlying [`HTTPHeader`] for advanced use.
    pub fn headers(&mut self) -> &mut HTTPHeader {
        &mut self.message.headers
    }

    /// Sets the response body from raw bytes and updates `Content-Length`.
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.message.headers.content_length = Some(body.len() as u64);
        self.message.body = Some(body);
    }

    /// Sets the response body from a `String` and updates `Content-Length`.
    pub fn set_body_string(&mut self, body: String) {
        self.set_body(body.into_bytes());
    }

    /// Returns the current body, or `None` if no body has been set.
    pub fn body(&self) -> Option<Body> {
        self.message.body.as_ref().map(|b| Body::new(b.clone()))
    }
}

// ── Standard headers ──────────────────────────────────────────────────────────

impl Response {
    /// Sets the `Date` header to the current UTC time.
    pub fn set_date_now(&mut self) {
        self.message.headers.set_date_now();
    }

    /// Sets the `Server` header (e.g. `"SunWeb/0.3"`).
    pub fn set_server(&mut self, server_name: &str) {
        self.message.headers.set_server(server_name);
    }

    /// Sets the `Location` header. Used by [`Response::redirect`] internally.
    pub fn set_location(&mut self, url: &str) {
        self.message.headers.set_location(url);
    }

    /// Sets the `ETag` header, wrapping the value in quotes.
    pub fn set_etag(&mut self, etag: &str) {
        self.message.headers.set_etag(etag);
    }

    /// Sets the `Content-Encoding` header from a [`ContentEncoding`] value.
    pub fn set_content_encoding(&mut self, encoding: ContentEncoding) {
        self.message.headers.set_content_encoding(encoding);
    }

    /// Sets the `Transfer-Encoding` header from a [`TransferEncoding`] value.
    pub fn set_transfer_encoding(&mut self, encoding: TransferEncoding) {
        self.message.headers.set_transfer_encoding(encoding);
    }
}

// ── Cache headers ─────────────────────────────────────────────────────────────

impl Response {
    /// Sets the `Cache-Control` header from a [`CacheControl`] directive.
    ///
    /// Use [`CacheControl::Multiple`] to combine several directives:
    /// ```rust,ignore
    /// res.set_cache_control(CacheControl::Multiple(vec![
    ///     CacheControl::NoCache,
    ///     CacheControl::NoStore,
    ///     CacheControl::MustRevalidate,
    /// ]));
    /// ```
    pub fn set_cache_control(&mut self, directive: CacheControl) {
        self.message.headers.set_cache_control(directive);
    }
}

// ── Security headers ──────────────────────────────────────────────────────────

impl Response {
    /// Sets `X-Content-Type-Options: nosniff` to prevent MIME sniffing.
    pub fn set_nosniff(&mut self) {
        self.message.headers.set_nosniff();
    }

    /// Sets `X-Frame-Options` to guard against clickjacking.
    pub fn set_frame_options(&mut self, option: FrameOption) {
        self.message.headers.set_frame_options(option);
    }

    /// Sets `Strict-Transport-Security` to enforce HTTPS.
    ///
    /// Pass `include_subdomains: true` to also cover all subdomains.
    pub fn set_hsts(&mut self, max_age_seconds: u64, include_subdomains: bool) {
        self.message
            .headers
            .set_hsts(max_age_seconds, include_subdomains);
    }

    /// Sets `Content-Security-Policy` from a [`CspBuilder`].
    ///
    /// ```rust,ignore
    /// res.set_csp(
    ///     CspBuilder::new()
    ///         .directive(CspDirective::DefaultSrc(vec!["'self'".into()]))
    ///         .directive(CspDirective::ImgSrc(vec!["'self'".into(), "https:".into()]))
    /// );
    /// ```
    pub fn set_csp(&mut self, policy: CspBuilder) {
        self.message.headers.set_csp(policy);
    }

    /// Sets `X-XSS-Protection`. Prefer CSP in modern browsers — this is a
    /// legacy header included for compatibility.
    pub fn set_xss_protection(&mut self, enabled: bool) {
        self.message.headers.set_xss_protection(enabled);
    }

    /// Sets the `Referrer-Policy` header.
    pub fn set_referrer_policy(&mut self, policy: ReferrerPolicy) {
        self.message.headers.set_referrer_policy(policy);
    }

    /// Applies a sensible default security header bundle:
    /// `X-Content-Type-Options`, `X-Frame-Options: Deny`,
    /// `X-XSS-Protection`, `Content-Security-Policy: default-src 'self'`,
    /// and `Referrer-Policy: strict-origin-when-cross-origin`.
    pub fn apply_security_headers(&mut self) {
        self.message.headers.apply_security_headers();
    }
}

// ── CORS headers ──────────────────────────────────────────────────────────────

impl Response {
    /// Sets `Access-Control-Allow-Origin`. Use `"*"` to allow all origins.
    pub fn set_cors_origin(&mut self, origin: &str) {
        self.message.headers.set_cors_origin(origin);
    }

    /// Sets `Access-Control-Allow-Methods` from a slice of method strings.
    pub fn set_cors_methods(&mut self, methods: &[&str]) {
        self.message.headers.set_cors_methods(methods);
    }

    /// Sets `Access-Control-Allow-Headers` from a slice of header names.
    pub fn set_cors_headers(&mut self, headers: &[&str]) {
        self.message.headers.set_cors_headers(headers);
    }

    /// Sets `Access-Control-Max-Age` — how long preflight results may be cached.
    pub fn set_cors_max_age(&mut self, seconds: u64) {
        self.message.headers.set_cors_max_age(seconds);
    }

    /// Sets `Access-Control-Allow-Credentials: true` when `allow` is `true`.
    pub fn set_cors_credentials(&mut self, allow: bool) {
        self.message.headers.set_cors_credentials(allow);
    }

    /// Applies permissive CORS headers allowing all origins and methods.
    ///
    /// ⚠️ **Development only** — do not use in production.
    pub fn apply_cors_permissive(&mut self) {
        self.message.headers.apply_cors_permissive();
    }
}

// ── Content-Type ──────────────────────────────────────────────────────────────

impl Response {
    /// Sets the `Content-Type` header.
    pub fn set_content_type(&mut self, content_type: ContentType) {
        self.message.headers.content_type = content_type;
    }

    /// Returns the currently set `Content-Type`.
    pub fn content_type(&self) -> &ContentType {
        &self.message.headers.content_type
    }

    /// `Content-Type: application/json`
    pub fn set_json(&mut self) {
        self.set_content_type(ContentType::Application(ApplicationSubType::Json));
    }

    /// `Content-Type: text/html`
    pub fn set_html(&mut self) {
        self.set_content_type(ContentType::Text(TextSubType::Html));
    }

    /// `Content-Type: text/plain`
    pub fn set_text(&mut self) {
        self.set_content_type(ContentType::Text(TextSubType::Plain));
    }

    /// `Content-Type: video/*`
    pub fn set_video(&mut self, subtype: VideoSubType) {
        self.set_content_type(ContentType::Video(subtype));
    }

    /// `Content-Type: audio/*`
    pub fn set_audio(&mut self, subtype: AudioSubType) {
        self.set_content_type(ContentType::Audio(subtype));
    }

    /// `Content-Type: image/*`
    pub fn set_image(&mut self, subtype: ImageSubType) {
        self.set_content_type(ContentType::Image(subtype));
    }
}

// ── Serialization ─────────────────────────────────────────────────────────────

impl Response {
    /// Serializes the response into an HTTP/1.1 byte stream ready to write to
    /// a `TcpStream`.
    ///
    /// Writes the status line, `Content-Type`, `Content-Length`, `Connection`,
    /// all custom headers, and the body (if any).
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut response = format!(
            "{} {} {}\r\n",
            self.message.http_version,
            self.status_code.as_u16(),
            self.status_code
        );

        response.push_str(&format!(
            "Content-Type: {}\r\n",
            self.message.headers.content_type
        ));

        if let Some(len) = self.message.headers.content_length {
            response.push_str(&format!("Content-Length: {}\r\n", len));
        }

        response.push_str(&format!(
            "Connection: {}\r\n",
            self.message.headers.connection
        ));

        response.push_str(&self.message.headers.as_str());
        response.push_str("\r\n");

        let mut bytes = response.into_bytes();

        if let Some(body) = &self.message.body {
            bytes.extend_from_slice(body);
        }

        bytes
    }
}
