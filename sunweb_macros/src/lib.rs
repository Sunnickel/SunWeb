//! Procedural macros for the SunWeb framework.
//!
//! You should not depend on this crate directly — use [`sunweb`] instead,
//! which re-exports everything from here.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream}, parse_macro_input, DeriveInput, Expr, ItemFn, LitInt,
    LitStr,
    Token,
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

struct ParamArgs {
    name: LitStr,
    _comma: Token![,],
    ty: syn::Type,
}
impl Parse for ParamArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            _comma: input.parse()?,
            ty: input.parse()?,
        })
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

#[proc_macro_attribute]
pub fn param(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ParamArgs { name, ty, .. } = parse_macro_input!(attr as ParamArgs);
    let mut func = parse_macro_input!(item as ItemFn);

    let var_ident = syn::Ident::new(&name.value(), name.span());

    let parse_stmt: syn::Stmt = syn::parse_quote! {
            let #var_ident: #ty = match req.param(#name).map(|s| s.parse::<#ty>()) {
                Some(Ok(v)) => v,
                _ => return ::std::convert::Into::<sunweb::HtmlResponse>::into(
                    sunweb::HtmlResponse::status("", 400.into())
                ),
            };
        };

    // Collect already-injected param idents by scanning stmts.
    let mut accumulated: Vec<(String, syn::Ident)> = Vec::new();
    for stmt in &func.block.stmts {
        let s = quote!(#stmt).to_string();
        if s.contains("req . param (") && !s.contains("__sunweb_params") {
            if let syn::Stmt::Local(local) = stmt {
                if let syn::Pat::Type(pt) = &local.pat {
                    if let syn::Pat::Ident(pi) = &*pt.pat {
                        let expr_str = quote!(#local).to_string();
                        if let Some(start) = expr_str.find("req . param (\"") {
                            let rest = &expr_str[start + 14..];
                            if let Some(end) = rest.find('"') {
                                let param_name = rest[..end].to_string();
                                accumulated.push((param_name, pi.ident.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    accumulated.push((name.value(), var_ident.clone()));

    let entries = accumulated.iter().map(|(n, i)| {
        quote! { (#n, sunweb::Value::from(#i as i64)) }
    });

    let params_stmt: syn::Stmt = syn::parse_quote! {
        let __sunweb_params: &[(&str, sunweb::Value)] = &[ #(#entries),* ];
    };

    func.block
        .stmts
        .retain(|s| !quote!(#s).to_string().contains("__sunweb_params"));

    func.block.stmts.insert(0, params_stmt);
    func.block.stmts.insert(0, parse_stmt);

    TokenStream::from(quote! { #func })
}

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

#[cfg(feature = "templating")]
#[proc_macro]
pub fn render(input: TokenStream) -> TokenStream {
    let (template, explicit_ctx) = {
        let input2 = input.clone();
        if let Ok(r) = syn::parse::<RenderInput>(input2) {
            (r.template, Some(r.context))
        } else {
            let template: Expr = syn::parse(input).expect("render! expects a template expression");
            (template, None)
        }
    };

    let expanded = match explicit_ctx {
        Some(ctx) => quote! {{
            for (k, v) in __sunweb_params {
                #ctx.insert((*k).into(), v.clone());
            }
            sunweb::render_response(#template.as_str(), &#ctx)
        }},
        None => quote! {{
            let mut __ctx: sunweb::Context = ::std::collections::HashMap::new();
            for (k, v) in __sunweb_params {
                __ctx.insert((*k).into(), v.clone());
            }
            sunweb::render_response(#template.as_str(), &__ctx)
        }},
    };

    expanded.into()
}

// ── shared helper ────────────────────────────────────────────────────────────

fn method_route(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(attr as LitStr);
    let mut func = parse_macro_input!(item as ItemFn);
    let fn_name = &func.sig.ident;
    let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());

    // Inject empty __sunweb_params fallback if #[param] hasn't already done so.
    let has_params = func
        .block
        .stmts
        .iter()
        .any(|s| quote!(#s).to_string().contains("__sunweb_params"));

    if !has_params {
        let fallback: syn::Stmt = syn::parse_quote! {
            let __sunweb_params: &[(&str, sunweb::Value)] = &[];
        };
        func.block.stmts.insert(0, fallback);
    }

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
