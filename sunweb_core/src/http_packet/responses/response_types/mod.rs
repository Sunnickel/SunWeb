use crate::http_packet::header::connection::ConnectionType;
use crate::http_packet::header::content_types::image::ImageSubType;
use crate::http_packet::responses::status_code::StatusCode;
use crate::http_packet::responses::IntoResponse;
use crate::Response;

// ── Traits ────────────────────────────────────────────────────────────────────

pub trait TextResponse: IntoResponse + Sized {
    fn ok(data: impl Into<String>) -> Self;
    fn status(data: impl Into<String>, status_code: StatusCode) -> Self;
}

pub trait BinaryResponse: IntoResponse + Sized {
    fn ok(data: Vec<u8>) -> Self;
    fn status(data: Vec<u8>, status_code: StatusCode) -> Self;
}

// ── HTML ──────────────────────────────────────────────────────────────────────

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

pub struct ImageResponse {
    data: Vec<u8>,
    subtype: ImageSubType,
    status_code: StatusCode,
}

impl ImageResponse {
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

pub struct RedirectResponse {
    location: String,
    permanent: bool,
}

impl RedirectResponse {
    pub fn temporary(location: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            permanent: false,
        }
    }
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
