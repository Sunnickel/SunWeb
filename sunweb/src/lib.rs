pub use sunweb_core::*;

#[doc(inline)]
pub use sunweb_core::{parse_addr, AppBuilder, HTTPMethod, HTTPRequest, Response};

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
    delete, error_page, get, head, middleware, options, param, patch, post, proxy, put, static_files,
    App,
};

#[cfg(feature = "templating")]
#[doc(inline)]
pub use sunweb_templating::{render_response, Context, Value};

pub use inventory;
