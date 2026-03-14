use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, DeriveInput, ItemFn, LitInt, LitStr, Token,
};

struct StaticArgs {
    path: LitStr,
    _c1: Token![,],
    folder: LitStr,
}
impl Parse for StaticArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self { path: input.parse()?, _c1: input.parse()?, folder: input.parse()? })
    }
}

struct ErrorArgs {
    status_code: LitInt,
}
impl Parse for ErrorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self { status_code: input.parse()? })
    }
}

struct ProxyArgs {
    path: LitStr,
    _c1: Token![,],
    external: LitStr,
}
impl Parse for ProxyArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self { path: input.parse()?, _c1: input.parse()?, external: input.parse()? })
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
            Ok(Self { route: Some(input.parse()?) })
        }
    }
}

// ── per-method shorthand macros ──────────────────────────────────────────────

#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("GET", attr, item)
}
#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("POST", attr, item)
}
#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("PUT", attr, item)
}
#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("DELETE", attr, item)
}
#[proc_macro_attribute]
pub fn patch(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("PATCH", attr, item)
}
#[proc_macro_attribute]
pub fn head(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("HEAD", attr, item)
}
#[proc_macro_attribute]
pub fn options(attr: TokenStream, item: TokenStream) -> TokenStream {
    method_route("OPTIONS", attr, item)
}

/// Registers a static file folder. Attach to a dummy struct or use standalone.
///
/// ```rust
/// #[static_files("/assets", "./public")]
/// struct Assets;
/// ```
#[proc_macro_attribute]
pub fn static_files(attr: TokenStream, item: TokenStream) -> TokenStream {
    let StaticArgs { path, folder, .. } = parse_macro_input!(attr as StaticArgs);
    let item: proc_macro2::TokenStream = item.into();
    TokenStream::from(quote! {
        #item
        sunweb::inventory::submit! {
            sunweb::RouteRegistration::Static { path: #path, folder: #folder }
        }
    })
}

/// Registers a custom error page handler for a status code.
///
/// ```rust
/// #[error_page(404)]
/// fn not_found(req: &HTTPRequest) -> Response { Response::html("<h1>Not Found</h1>") }
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

/// Registers a reverse-proxy route to an external URL.
///
/// ```rust
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

/// Derive macro that adds `run(addr)` to your app struct.
///
/// ```rust
/// #[derive(App)]
/// struct MyApp;
///
/// fn main() {
///     MyApp::run("0.0.0.0:8080");
/// }
/// ```
#[proc_macro_derive(App)]
pub fn derive_app(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    TokenStream::from(quote! {
        impl #name {
            pub fn run(addr: &str) {
                let (host, port) = {
                    let (host_str, port_str) = addr
                        .rsplit_once(':')
                        .unwrap_or_else(|| panic!("Invalid addr `{}` — expected host:port", addr));

                    let port: u16 = port_str
                        .parse()
                        .unwrap_or_else(|_| panic!("Invalid port `{}`", port_str));

                    let parts: Vec<u8> = host_str
                        .split('.')
                        .map(|seg| seg.parse::<u8>()
                            .unwrap_or_else(|_| panic!("Invalid host segment `{}`", seg)))
                        .collect();

                    match parts.as_slice() {
                        [a, b, c, d] => ([*a, *b, *c, *d], port),
                        _ => panic!("Host `{}` must be IPv4", host_str),
                    }
                };

                sunweb::AppBuilder::new(host, port).run();
            }
        }
    })
}

/// Registers a request middleware. Function must take `&mut HTTPRequest`.
///
/// ```rust
/// #[middleware]                    // applies to all routes
/// fn auth(req: &mut HTTPRequest) { ... }
///
/// #[middleware("/api")]            // applies only to /api/*
/// fn api_auth(req: &mut HTTPRequest) { ... }
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