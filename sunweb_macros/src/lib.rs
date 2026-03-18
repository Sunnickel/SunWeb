//! Procedural macros for the SunWeb framework.
//!
//! You should not depend on this crate directly — use [`sunweb`] instead,
//! which re-exports everything from here.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, Expr, ItemFn, LitInt, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct StaticArgs {
    path: LitStr,
    _c1: Token![,],
    folder: LitStr,
}
impl Parse for StaticArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.parse()?,
            _c1: input.parse()?,
            folder: input.parse()?,
        })
    }
}

struct ErrorArgs {
    status_code: LitInt,
}
impl Parse for ErrorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            status_code: input.parse()?,
        })
    }
}

struct ProxyArgs {
    path: LitStr,
    _c1: Token![,],
    external: LitStr,
}
impl Parse for ProxyArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.parse()?,
            _c1: input.parse()?,
            external: input.parse()?,
        })
    }
}

struct MiddlewareArgs {
    route: Option<LitStr>,
}
impl Parse for MiddlewareArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self { route: None })
        } else {
            Ok(Self {
                route: Some(input.parse()?),
            })
        }
    }
}

#[cfg(feature = "templating")]
struct RenderInput {
    template: Expr,
    _comma: Token![,],
    context: Expr,
}
#[cfg(feature = "templating")]
impl Parse for RenderInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(RenderInput {
            template: input.parse()?,
            _comma: input.parse()?,
            context: input.parse()?,
        })
    }
}

// ── per-method shorthand macros ──────────────────────────────────────────────

/// Registers a function as a `GET` route handler.
///
/// The function can be sync or async and must take an [`HTTPRequest`] and
/// return any type that implements `Into<Response>`.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{get, HTTPRequest, Response};
///
/// #[get("/hello")]
/// async fn hello(req: HTTPRequest) -> Response {
///     Response::text("Hello, world!")
/// }
/// ```
#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("GET", attr, item)
}

/// Registers a function as a `POST` route handler.
///
/// The function can be sync or async and must take an [`HTTPRequest`] and
/// return any type that implements `Into<Response>`.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{post, HTTPRequest, Response};
///
/// #[post("/submit")]
/// async fn submit(req: HTTPRequest) -> Response {
///     Response::text("Received!")
/// }
/// ```
#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("POST", attr, item)
}

/// Registers a function as a `PUT` route handler.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{put, HTTPRequest, Response};
///
/// #[put("/item")]
/// async fn update_item(req: HTTPRequest) -> Response {
///     Response::text("Updated!")
/// }
/// ```
#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("PUT", attr, item)
}

/// Registers a function as a `DELETE` route handler.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{delete, HTTPRequest, Response};
///
/// #[delete("/item")]
/// async fn remove_item(req: HTTPRequest) -> Response {
///     Response::text("Deleted!")
/// }
/// ```
#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("DELETE", attr, item)
}

/// Registers a function as a `PATCH` route handler.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{patch, HTTPRequest, Response};
///
/// #[patch("/item")]
/// async fn patch_item(req: HTTPRequest) -> Response {
///     Response::text("Patched!")
/// }
/// ```
#[proc_macro_attribute]
pub fn patch(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("PATCH", attr, item)
}

/// Registers a function as a `HEAD` route handler.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{head, HTTPRequest, Response};
///
/// #[head("/ping")]
/// fn ping(req: HTTPRequest) -> Response {
///     Response::no_content()
/// }
/// ```
#[proc_macro_attribute]
pub fn head(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("HEAD", attr, item)
}

/// Registers a function as an `OPTIONS` route handler.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{options, HTTPRequest, Response};
///
/// #[options("/resource")]
/// fn preflight(req: HTTPRequest) -> Response {
///     Response::no_content()
/// }
/// ```
#[proc_macro_attribute]
pub fn options(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("OPTIONS", attr, item)
}

/// Serves all files inside a folder under a URL path prefix.
///
/// Takes a URL path prefix and a local folder path. Any file inside the
/// folder is served at `<path>/<filename>`.
///
/// # Example
/// ```rust,ignore
/// use sunweb::static_files;
///
/// #[static_files("/assets", "./public")]
/// struct Assets;
/// ```
#[proc_macro_attribute]
pub fn static_files(attr: TokenStream, item: TokenStream) -> TokenStream {
    let StaticArgs { path, folder, .. } = parse_macro_input!(attr as StaticArgs);
    let item: proc_macro2::TokenStream = item.into();
    TokenStream::from(quote! {
        #[allow(dead_code)]
        #item
        sunweb::inventory::submit! {
            sunweb::RouteRegistration::Static { path: #path, folder: #folder }
        }
    })
}

/// Registers a custom error page handler for a specific HTTP status code.
///
/// The handler receives the original [`HTTPRequest`] and must return a
/// [`Response`]. Both sync and async functions are supported.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{error_page, HTTPRequest, Response};
///
/// #[error_page(404)]
/// fn not_found(req: HTTPRequest) -> Response {
///     Response::html("<h1>404 — Page Not Found</h1>")
/// }
///
/// #[error_page(500)]
/// async fn server_error(req: HTTPRequest) -> Response {
///     Response::html("<h1>500 — Internal Server Error</h1>")
/// }
/// ```
#[proc_macro_attribute]
pub fn error_page(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ErrorArgs { status_code } = parse_macro_input!(attr as ErrorArgs);
    let func = parse_macro_input!(item as ItemFn);
    let fn_name = &func.sig.ident;

    let handler = if func.sig.asyncness.is_some() {
        quote! { |req| ::std::boxed::Box::pin(async {
            ::std::convert::Into::<sunweb::Response>::into(#fn_name(req).await)
        })}
    } else {
        quote! { |req| ::std::boxed::Box::pin(async {
            ::std::convert::Into::<sunweb::Response>::into(#fn_name(req))
        })}
    };

    TokenStream::from(quote! {
        #func
        sunweb::inventory::submit! {
            sunweb::RouteRegistration::Error {
                status_code: #status_code,
                handler: #handler,
            }
        }
    })
}

/// Registers a reverse-proxy route that forwards requests to an external URL.
///
/// All requests to `path` (and any sub-paths) are forwarded to `external`,
/// with the response passed back to the client transparently.
///
/// # Example
/// ```rust,ignore
/// use sunweb::proxy;
///
/// #[proxy("/api", "http://localhost:3000")]
/// struct ApiProxy;
/// ```
#[proc_macro_attribute]
pub fn proxy(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ProxyArgs { path, external, .. } = parse_macro_input!(attr as ProxyArgs);
    let item: proc_macro2::TokenStream = item.into();
    TokenStream::from(quote! {
        #item
        sunweb::inventory::submit! {
            sunweb::RouteRegistration::Proxy { path: #path, external: #external }
        }
    })
}

/// Derive macro that wires up your app struct with a [`AppBuilder`] entry point.
///
/// Adds a `builder()` associated function to the annotated struct, which
/// returns a configured [`AppBuilder`] ready to call `.run()` on.
///
/// # Example
/// ```rust,ignore
/// use sunweb::*;
///
/// #[derive(App)]
/// struct MainApp;
///
/// fn main() {
///     MainApp::builder()
///         .http("0.0.0.0:80")
///         .https("0.0.0.0:443")
///         .cert(
///             "./example_app/resources/cert/key.pem",
///             "./example_app/resources/cert/cert.pem",
///         )
///         .run();
/// }
/// ```
#[proc_macro_derive(App)]
pub fn derive_app(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    TokenStream::from(quote! {
        impl #name {
            pub fn builder() -> sunweb::AppBuilder {
                sunweb::AppBuilder::new()
            }
        }
    })
}

/// Registers a middleware function that runs before or after route handlers.
///
/// With no argument, the middleware applies to **all** routes. Pass a path
/// prefix string to scope it to a specific route subtree.
///
/// The function signature determines the middleware type:
/// - `fn(&mut HTTPRequest)` — request middleware, runs before the handler
/// - `fn(&mut Response)` — response middleware, runs after the handler
/// - `fn(&mut HTTPRequest, &mut Response)` — runs at both stages
///
/// # Example
/// ```rust,ignore
/// use sunweb::{middleware, HTTPRequest, Response};
///
/// // Applies to every route
/// #[middleware]
/// fn log_request(req: &mut HTTPRequest) {
///     println!("→ {}", req.path());
/// }
///
/// // Applies only to routes under /api
/// #[middleware("/api")]
/// fn require_auth(req: &mut HTTPRequest) {
///     // reject unauthenticated requests
/// }
///
/// // Response middleware
/// #[middleware]
/// fn add_cors(res: &mut Response) {
///     res.set_header("Access-Control-Allow-Origin", "*");
/// }
/// ```
#[proc_macro_attribute]
pub fn middleware(attr: TokenStream, item: TokenStream) -> TokenStream {
    let MiddlewareArgs { route } = parse_macro_input!(attr as MiddlewareArgs);
    let func = parse_macro_input!(item as ItemFn);
    let fn_name = &func.sig.ident;

    let inputs = &func.sig.inputs;
    let param_count = inputs.len();

    let route_tokens = match &route {
        Some(r) => quote! { Some(#r) },
        None => quote! { None },
    };

    let registration = match param_count {
        1 => {
            let is_response = func.sig.inputs.iter().any(|arg| {
                if let syn::FnArg::Typed(pat) = arg {
                    quote!(#pat).to_string().contains("Response")
                } else {
                    false
                }
            });

            if is_response {
                quote! {
                    sunweb::MiddlewareRegistration::Response {
                        route: #route_tokens,
                        handler: #fn_name,
                    }
                }
            } else {
                quote! {
                    sunweb::MiddlewareRegistration::Request {
                        route: #route_tokens,
                        handler: #fn_name,
                    }
                }
            }
        }
        2 => quote! {
            sunweb::MiddlewareRegistration::RequestResponse {
                route: #route_tokens,
                handler: #fn_name,
            }
        },
        _ => panic!("Middleware must take 1 or 2 parameters"),
    };

    TokenStream::from(quote! {
        #func
        sunweb::inventory::submit! {
            #registration
        }
    })
}

/// Renders a template with a context and returns a [`Response`].
///
/// Takes a template name (relative to your configured template directory)
/// and a [`Context`] populated with template variables.
///
/// This macro is only available with the **`templating`** feature enabled.
///
/// # Example
/// ```rust,ignore
/// use sunweb::{get, render, HTTPRequest, Response, Context};
///
/// #[get("/")]
/// async fn index(req: HTTPRequest) -> Response {
///     let mut ctx = Context::new();
///     ctx.insert("title", "Home");
///     ctx.insert("user", "Alice");
///     render!("index.html", ctx)
/// }
/// ```
#[cfg(feature = "templating")]
#[proc_macro]
pub fn render(input: TokenStream) -> TokenStream {
    let RenderInput {
        template, context, ..
    } = syn::parse_macro_input!(input as RenderInput);

    let expanded = quote! {
        {
            let __template: &str = #template.as_str();
            let __context = &#context;

            sunweb::render_response(__template, __context)
        }
    };

    expanded.into()
}

// ── shared helper ────────────────────────────────────────────────────────────

fn method_route(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(attr as LitStr);
    let func = parse_macro_input!(item as ItemFn);
    let fn_name = &func.sig.ident;
    let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());

    let handler = if func.sig.asyncness.is_some() {
        quote! { |req| ::std::boxed::Box::pin(async {
            ::std::convert::Into::<sunweb::Response>::into(#fn_name(req).await)
        })}
    } else {
        quote! { |req| ::std::boxed::Box::pin(async {
            ::std::convert::Into::<sunweb::Response>::into(#fn_name(req))
        })}
    };
    TokenStream::from(quote! {
        #func
        sunweb::inventory::submit! {
            sunweb::RouteRegistration::Custom {
                method: sunweb::HTTPMethod::#method_ident,
                path: #path,
                handler: #handler,
            }
        }
    })
}
