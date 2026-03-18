use crate::Response;
use crate::http_packet::header::connection::ConnectionType;
use crate::http_packet::header::content_types::image::ImageSubType;
use crate::http_packet::responses::IntoResponse;
use crate::http_packet::responses::status_code::StatusCode;

// ── Traits ────────────────────────────────────────────────────────────────────

/// Implemented by response types that carry a text body.
///
/// Both `ok` and `status` accept anything that converts to a `String`,
/// so you can pass `&str`, `String`, or `format!(...)` directly.
pub trait TextResponse: IntoResponse + Sized {
    /// Creates a `200 OK` response with the given text body.
    fn ok(data: impl Into<String>) -> Self;
    /// Creates a response with the given text body and status code.
    fn status(data: impl Into<String>, status_code: StatusCode) -> Self;
}

/// Implemented by response types that carry a binary body.
pub trait BinaryResponse: IntoResponse + Sized {
    /// Creates a `200 OK` response with the given binary body.
    fn ok(data: Vec<u8>) -> Self;
    /// Creates a response with the given binary body and status code.
    fn status(data: Vec<u8>, status_code: StatusCode) -> Self;
}

// ── HTML ──────────────────────────────────────────────────────────────────────

/// A `Content-Type: text/html` response.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{get, HTTPRequest, HtmlResponse, TextResponse};
///
/// #[get("/")]
/// fn index(req: HTTPRequest) -> HtmlResponse {
///     HtmlResponse::ok("<h1>Hello</h1>")
/// }
///
/// #[get("/error")]
/// fn error(req: HTTPRequest) -> HtmlResponse {
///     HtmlResponse::status("<h1>Gone</h1>", StatusCode::Gone)
/// }
/// ```
pub struct HtmlResponse {
    data: String,
    status_code: StatusCode,
}

impl TextResponse for HtmlResponse {
    fn ok(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            status_code: StatusCode::Ok,
        }
    }
    fn status(data: impl Into<String>, status_code: StatusCode) -> Self {
        Self {
            data: data.into(),
            status_code,
        }
    }
}

impl IntoResponse for HtmlResponse {
    fn into_response(self) -> Response {
        let mut r = Response::new(self.status_code);
        r.set_body_string(self.data);
        r.headers().connection = ConnectionType::KeepAlive;
        r.set_html();
        r
    }
}

impl From<HtmlResponse> for Response {
    fn from(r: HtmlResponse) -> Response {
        r.into_response()
    }
}

// ── JSON ──────────────────────────────────────────────────────────────────────

/// A `Content-Type: application/json` response.
///
/// Pass a pre-serialized JSON string, or use `serde_json::to_string` to
/// serialize a struct first.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{post, HTTPRequest, JsonResponse, TextResponse};
///
/// #[post("/echo")]
/// fn echo(req: HTTPRequest) -> JsonResponse {
///     JsonResponse::ok(r#"{"ok": true}"#)
/// }
/// ```
pub struct JsonResponse {
    data: String,
    status_code: StatusCode,
}

impl TextResponse for JsonResponse {
    fn ok(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            status_code: StatusCode::Ok,
        }
    }
    fn status(data: impl Into<String>, status_code: StatusCode) -> Self {
        Self {
            data: data.into(),
            status_code,
        }
    }
}

impl IntoResponse for JsonResponse {
    fn into_response(self) -> Response {
        let mut r = Response::new(self.status_code);
        r.set_body_string(self.data);
        r.headers().connection = ConnectionType::KeepAlive;
        r.set_json();
        r
    }
}

impl From<JsonResponse> for Response {
    fn from(r: JsonResponse) -> Response {
        r.into_response()
    }
}

// ── Plain text ────────────────────────────────────────────────────────────────

/// A `Content-Type: text/plain` response.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{get, HTTPRequest, PlainTextResponse, TextResponse};
///
/// #[get("/ping")]
/// fn ping(req: HTTPRequest) -> PlainTextResponse {
///     PlainTextResponse::ok("pong")
/// }
/// ```
pub struct PlainTextResponse {
    data: String,
    status_code: StatusCode,
}

impl TextResponse for PlainTextResponse {
    fn ok(data: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            status_code: StatusCode::Ok,
        }
    }
    fn status(data: impl Into<String>, status_code: StatusCode) -> Self {
        Self {
            data: data.into(),
            status_code,
        }
    }
}

impl IntoResponse for PlainTextResponse {
    fn into_response(self) -> Response {
        let mut r = Response::new(self.status_code);
        r.set_body_string(self.data);
        r.headers().connection = ConnectionType::KeepAlive;
        r.set_text();
        r
    }
}

impl From<PlainTextResponse> for Response {
    fn from(r: PlainTextResponse) -> Response {
        r.into_response()
    }
}

// ── Image ─────────────────────────────────────────────────────────────────────

/// A `Content-Type: image/*` response carrying raw binary image data.
///
/// `ok` and `status` default to `image/png`. Use [`ImageResponse::new`] to
/// specify a different [`ImageSubType`].
///
/// # Example
/// ```rust,ignore
/// use sunweb::{get, HTTPRequest, ImageResponse, BinaryResponse};
/// use sunweb_core::http_packet::header::content_types::image::ImageSubType;
///
/// #[get("/logo")]
/// fn logo(req: HTTPRequest) -> ImageResponse {
///     let bytes = std::fs::read("logo.png").unwrap();
///     ImageResponse::ok(bytes)
/// }
///
/// #[get("/photo")]
/// fn photo(req: HTTPRequest) -> ImageResponse {
///     let bytes = std::fs::read("photo.jpg").unwrap();
///     ImageResponse::new(bytes, ImageSubType::Jpeg, StatusCode::Ok)
/// }
/// ```
pub struct ImageResponse {
    data: Vec<u8>,
    subtype: ImageSubType,
    status_code: StatusCode,
}

impl ImageResponse {
    /// Creates an image response with an explicit [`ImageSubType`] and status code.
    pub fn new(data: Vec<u8>, subtype: ImageSubType, status_code: StatusCode) -> Self {
        Self {
            data,
            subtype,
            status_code,
        }
    }
}

impl BinaryResponse for ImageResponse {
    fn ok(data: Vec<u8>) -> Self {
        Self::new(data, ImageSubType::Png, StatusCode::Ok)
    }
    fn status(data: Vec<u8>, status_code: StatusCode) -> Self {
        Self::new(data, ImageSubType::Png, status_code)
    }
}

impl IntoResponse for ImageResponse {
    fn into_response(self) -> Response {
        let mut r = Response::new(self.status_code);
        r.set_body(self.data);
        r.headers().connection = ConnectionType::KeepAlive;
        r.set_image(self.subtype);
        r
    }
}

impl From<ImageResponse> for Response {
    fn from(r: ImageResponse) -> Response {
        r.into_response()
    }
}

// ── Redirect ──────────────────────────────────────────────────────────────────

/// A redirect response — either `307 Temporary` or `308 Permanent`.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{get, HTTPRequest, RedirectResponse};
///
/// #[get("/old")]
/// fn old(req: HTTPRequest) -> RedirectResponse {
///     RedirectResponse::permanent("/new")
/// }
///
/// #[get("/dashboard")]
/// fn dashboard(req: HTTPRequest) -> RedirectResponse {
///     RedirectResponse::temporary("/login")
/// }
/// ```
pub struct RedirectResponse {
    location: String,
    permanent: bool,
}

impl RedirectResponse {
    /// `307 Temporary Redirect` to `location`.
    pub fn temporary(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            permanent: false,
        }
    }

    /// `308 Permanent Redirect` to `location`.
    pub fn permanent(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            permanent: true,
        }
    }
}

impl IntoResponse for RedirectResponse {
    fn into_response(self) -> Response {
        Response::redirect(&self.location, self.permanent)
    }
}

impl From<RedirectResponse> for Response {
    fn from(r: RedirectResponse) -> Response {
        r.into_response()
    }
}

// ── No Content ────────────────────────────────────────────────────────────────

/// A `204 No Content` response with no body.
///
/// Typically used for `DELETE` or `OPTIONS` handlers that have nothing to return.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{delete, HTTPRequest, NoContentResponse};
///
/// #[delete("/item")]
/// fn remove(req: HTTPRequest) -> NoContentResponse {
///     NoContentResponse
/// }
/// ```
pub struct NoContentResponse;

impl IntoResponse for NoContentResponse {
    fn into_response(self) -> Response {
        Response::new(StatusCode::NoContent)
    }
}

impl From<NoContentResponse> for Response {
    fn from(r: NoContentResponse) -> Response {
        r.into_response()
    }
}
