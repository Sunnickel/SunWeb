pub use sunweb_core::*;

#[doc(inline)]
pub use sunweb_core::{AppBuilder, HTTPMethod, HTTPRequest, Response, parse_addr};

#[doc(inline)]
pub use sunweb_core::{MiddlewareRegistration, RouteRegistration};

#[doc(inline)]
pub use sunweb_core::response_types::{
    BinaryResponse, HtmlResponse, ImageResponse, JsonResponse, NoContentResponse,
    PlainTextResponse, RedirectResponse, TextResponse,
};

#[cfg(feature = "templating")]
#[doc(inline)]
pub use sunweb_macros::render;

#[doc(inline)]
pub use sunweb_macros::{
    App, delete, error_page, get, head, middleware, options, patch, post, proxy, put, static_files,
};

#[cfg(feature = "templating")]
#[doc(inline)]
pub use sunweb_templating::{Context, Value, render_response};

pub use inventory;
