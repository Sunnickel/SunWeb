pub use sunweb_core::*;

pub use sunweb_core::{
    AppBuilder,
    HTTPMethod,
    HTTPRequest,
    Response,
    parse_addr,
};

pub use sunweb_core::{
    RouteRegistration,
    MiddlewareRegistration,
};

pub use sunweb_core::response_types::{
    TextResponse, BinaryResponse,
    HtmlResponse, JsonResponse, PlainTextResponse,
    ImageResponse, RedirectResponse, NoContentResponse,
};

pub use sunweb_macros::{
    App,
    get, post, put, delete, patch, head, options,
    static_files,
    error_page,
    proxy,
    middleware
};

pub use inventory;