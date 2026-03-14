use crate::http_packet::header::content_types::application::ApplicationSubType;
use crate::http_packet::header::content_types::audio::AudioSubType;
use crate::http_packet::header::content_types::image::ImageSubType;
use crate::http_packet::header::content_types::text::TextSubType;
use crate::http_packet::header::content_types::video::VideoSubType;
use crate::http_packet::header::content_types::ContentType;
use crate::http_packet::header::headers::frame_option::FrameOption;
use crate::http_packet::header::HTTPHeader;
use crate::http_packet::responses::status_code::StatusCode;
use crate::http_packet::HTTPMessage;
use std::collections::HashMap;
use crate::http_packet::body::Body;

pub mod response_types;
pub mod status_code;

/// A convenient wrapper around an [`HTTPMessage`] that couples it with a
/// [`StatusCode`].
///
/// The type is cheap to clone (all data is heap-allocated or copy-on-write)
/// and is intended to be mutated until the response is ready to be sent.
#[derive(Clone, Debug)]
pub struct Response {
    /// The three-digit status code that will appear in the first line of the response.
    pub status_code: StatusCode,
    pub(crate) message: HTTPMessage,
}

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

// -------------------- Constructors --------------------
impl Response {
    /// Creates a blank response with the supplied status code.
    ///
    /// No headers (except the mandatory ones added later by `to_bytes`) and no body
    ///  is set.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = HTTPResponse::new(StatusCode::ImATeapot);
    /// assert_eq!(resp.status_code.as_u16(), 418);
    /// ```
    pub fn new(status_code: StatusCode) -> Self {
        let headers = HTTPHeader::new(HashMap::new());
        let message = HTTPMessage::new("HTTP/1.1".to_string(), headers);

        Self {
            status_code,
            message,
        }
    }

    /// Shorthand for [`Self::new(StatusCode::Ok)`].
    pub fn ok() -> Self {
        Self::new(StatusCode::Ok)
    }

    /// Shorthand for [`Self::new(StatusCode::NotFound)`].
    pub fn not_found() -> Self {
        Self::new(StatusCode::NotFound)
    }

    /// Shorthand for [`Self::new(StatusCode::InternalServerError)`].
    pub fn internal_error() -> Self {
        Self::new(StatusCode::InternalServerError)
    }

    /// Shorthand for [`Self::new(StatusCode::MethodNotAllowed)`].
    pub(crate) fn method_not_allowed() -> Self {
        Self::new(StatusCode::MethodNotAllowed)
    }

    /// Shorthand for [`Self::new(StatusCode::BadGateway)`].
    pub(crate) fn bad_gateway() -> Self {
        Self::new(StatusCode::BadGateway)
    }

    /// Builds a redirect response with the appropriate 3xx status code and a
    /// `Location` header.
    ///
    /// # Example
    ///
    /// ```
    /// let r = HTTPResponse::redirect("/login", /*permanent=*/false);
    /// assert_eq!(r.status_code, StatusCode::TemporaryRedirect);
    /// assert_eq!(r.get_header("location"), Some("/login".into()));
    /// ```
    pub fn redirect(location: &str, permanent: bool) -> Self {
        let status = if permanent {
            StatusCode::TemporaryRedirect
        } else {
            StatusCode::PermanentRedirect
        };
        let mut response = Self::new(status);
        response.set_location(location);
        response
    }
}

// Functions
impl Response {
    // ===== Header Delegation Methods =====

    /// Adds an arbitrary header to the response.
    ///
    /// If the header already exists, the new value is *appended* according to
    /// HTTP rules (comma-separated for most headers).
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.message.headers.add_header(key, value);
    }

    /// Returns the first value associated with the header name, if any.
    ///
    /// Matching is case-insensitive.
    pub fn get_header(&self, name: &str) -> Option<String> {
        self.message.headers.get_header(name)
    }

    /// Grants mutable access to the underlying [`HTTPHeader`] struct.
    ///
    /// This is useful when you want to call niche helper methods that are not
    /// surfaced through `HTTPResponse` itself.
    pub fn headers(&mut self) -> &mut HTTPHeader {
        &mut self.message.headers
    }

    // ===== Body Methods =====

    /// Replaces the response body with the supplied bytes and automatically
    /// sets the `Content-Length` header.
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.message.headers.content_length = Some(body.len() as u64);
        self.message.body = Some(body);
    }

    /// Convenience wrapper around [`set_body`](Self::set_body) that converts a
    /// `String` to bytes.
    pub fn set_body_string(&mut self, body: String) {
        self.set_body(body.into_bytes());
    }

    /// Returns a slice into the current body, if one has been set.
    pub fn body(&self) -> Option<Body> {
        self.message.body.as_ref().map(|b| Body::new(b.clone()))
    }


    // ===== Convenience Methods (delegating to HTTPHeader) =====

    /// Delegates to [`HTTPHeader::set_date_now`].
    pub fn set_date_now(&mut self) {
        self.message.headers.set_date_now();
    }

    /// Delegates to [`HTTPHeader::set_server`].
    pub fn set_server(&mut self, server_name: &str) {
        self.message.headers.set_server(server_name);
    }

    /// Delegates to [`HTTPHeader::set_location`].
    pub fn set_location(&mut self, url: &str) {
        self.message.headers.set_location(url);
    }

    /// Delegates to [`HTTPHeader::set_cache_control`].
    pub fn set_cache_control(&mut self, directive: &str) {
        self.message.headers.set_cache_control(directive);
    }

    /// Shorthand for `Cache-Control: no-cache, no-store, must-revalidate`.
    pub fn set_no_cache(&mut self) {
        self.message.headers.set_no_cache();
    }

    /// Shorthand for `Cache-Control: max-age=N`.
    pub fn set_max_age(&mut self, seconds: u64) {
        self.message.headers.set_max_age(seconds);
    }

    /// Delegates to [`HTTPHeader::set_etag`].
    pub fn set_etag(&mut self, etag: &str) {
        self.message.headers.set_etag(etag);
    }

    /// Delegates to [`HTTPHeader::set_content_encoding`].
    pub fn set_content_encoding(&mut self, encoding: &str) {
        self.message.headers.set_content_encoding(encoding);
    }

    /// Delegates to [`HTTPHeader::set_transfer_encoding`].
    pub fn set_transfer_encoding(&mut self, encoding: &str) {
        self.message.headers.set_transfer_encoding(encoding);
    }

    /// Adds `X-Content-Type-Options: nosniff`.
    pub fn set_nosniff(&mut self) {
        self.message.headers.set_nosniff();
    }

    /// Adds or overwrites the `X-Frame-Options` header.
    pub fn set_frame_options(&mut self, option: FrameOption) {
        self.message.headers.set_frame_options(option);
    }

    /// Adds the `Strict-Transport-Security` header.
    pub fn set_hsts(&mut self, max_age_seconds: u64, include_subdomains: bool) {
        self.message
            .headers
            .set_hsts(max_age_seconds, include_subdomains);
    }

    /// Adds or replaces the `Content-Security-Policy` header.
    pub fn set_csp(&mut self, policy: &str) {
        self.message.headers.set_csp(policy);
    }

    /// Adds `X-XSS-Protection: 1; mode=block` or disables it.
    pub fn set_xss_protection(&mut self, enabled: bool) {
        self.message.headers.set_xss_protection(enabled);
    }
    /// Applies a conservative set of security headers in one call.
    ///
    /// The current set is:
    /// - `X-Content-Type-Options: nosniff`
    /// - `X-Frame-Options: DENY`
    /// - `X-XSS-Protection: 1; mode=block`
    /// - `Content-Security-Policy: default-src 'self'`
    /// - `Strict-Transport-Security: max-age=31536000; includeSubDomains`
    pub fn apply_security_headers(&mut self) {
        self.message.headers.apply_security_headers();
    }

    /// Adds `Access-Control-Allow-Origin`.
    pub fn set_cors_origin(&mut self, origin: &str) {
        self.message.headers.set_cors_origin(origin);
    }

    /// Adds `Access-Control-Allow-Methods`.
    pub fn set_cors_methods(&mut self, methods: &[&str]) {
        self.message.headers.set_cors_methods(methods);
    }

    /// Adds `Access-Control-Allow-Headers`.
    pub fn set_cors_headers(&mut self, headers: &[&str]) {
        self.message.headers.set_cors_headers(headers);
    }

    /// Adds `Access-Control-Max-Age`.
    pub fn set_cors_max_age(&mut self, seconds: u64) {
        self.message.headers.set_cors_max_age(seconds);
    }

    /// Adds `Access-Control-Allow-Credentials`.
    pub fn set_cors_credentials(&mut self, allow: bool) {
        self.message.headers.set_cors_credentials(allow);
    }

    /// Applies the most permissive CORS policy:
    /// - `Access-Control-Allow-Origin: *`
    /// - `Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS`
    /// - `Access-Control-Allow-Headers: *`
    /// - `Access-Control-Allow-Credentials: true`
    pub fn apply_cors_permissive(&mut self) {
        self.message.headers.apply_cors_permissive();
    }

    // ===== Content-Type Methods =====

    /// Overwrites the `Content-Type` header with the supplied value.
    pub fn set_content_type(&mut self, content_type: ContentType) {
        self.message.headers.content_type = content_type;
    }

    /// Returns the currently set content type.
    pub fn content_type(&self) -> &ContentType {
        &self.message.headers.content_type
    }

    /// Shorthand for `Content-Type: application/json`.
    pub fn set_json(&mut self) {
        self.set_content_type(ContentType::Application(ApplicationSubType::Json));
    }

    /// Shorthand for `Content-Type: text/html`.
    pub fn set_html(&mut self) {
        self.set_content_type(ContentType::Text(TextSubType::Html));
    }

    /// Shorthand for `Content-Type: text/plain`.
    pub fn set_text(&mut self) {
        self.set_content_type(ContentType::Text(TextSubType::Plain));
    }

    /// Shorthand for `Content-Type: video/*`.
    pub fn set_video(&mut self, subtype: VideoSubType) {
        self.set_content_type(ContentType::Video(subtype));
    }

    /// Shorthand for `Content-Type: audio/*`.
    pub fn set_audio(&mut self, subtype: AudioSubType) {
        self.set_content_type(ContentType::Audio(subtype));
    }

    /// Shorthand for `Content-Type: image/*`.
    pub fn set_image(&mut self, subtype: ImageSubType) {
        self.set_content_type(ContentType::Image(subtype));
    }

    // ===== Response Building Methods =====

    /// Serializes the response into a valid HTTP/1.1 byte stream.
    ///
    /// The returned buffer contains the status line, all headers (including
    /// those set implicitly such as `Content-Length`) and, if present, the
    /// body. It is ready to be written directly to a `TcpStream`.
    ///
    /// # Example
    ///
    /// ```
    /// let mut r = HTTPResponse::ok();
    /// r.set_body_string("Hello".into());
    /// let bytes = r.to_bytes();
    /// assert!(bytes.starts_with(b"HTTP/1.1 200"));
    /// assert!(bytes.ends_with(b"Hello"));
    /// ```
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut response = format!(
            "{} {} {}\r\n",
            self.message.http_version,
            self.status_code.as_u16(),
            self.status_code.to_string()
        );

        // Add content-type and content-length
        response.push_str(&format!(
            "Content-Type: {}\r\n",
            self.message.headers.content_type.to_string()
        ));

        if let Some(len) = self.message.headers.content_length {
            response.push_str(&format!("Content-Length: {}\r\n", len));
        }

        response.push_str(&format!(
            "Connection: {}\r\n",
            self.message.headers.connection.to_string()
        ));

        // Add all other headers
        response.push_str(&self.message.headers.as_str());

        // End of headers
        response.push_str("\r\n");

        let mut bytes = response.into_bytes();

        // Add body if present
        if let Some(body) = &self.message.body {
            bytes.extend_from_slice(body);
        }

        bytes
    }
}
