# Project: sunweb
> Sunnickel | 04.10.2025
---
A lightweight, fast, and flexible HTTP/HTTPS web server written in Rust with domain routing, middleware support, and TLS
capabilities.

---

## Features

- **Macro-driven routing** — define routes with `#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`, `#[head]`, `#[options]`
- **HTTP & HTTPS** — built-in TLS via `rustls`, automatic HTTP → HTTPS redirect
- **Middleware** — request, response, and request+response middleware, scoped by path prefix
- **Static file serving** — `#[static_files("/assets", "./public")]`
- **Reverse proxy** — `#[proxy("/api", "http://localhost:3000")]`
- **Custom error pages** — `#[error_page(404)]`
- **Templating** *(optional feature)* — lightweight `{{ var }}`, `{% if %}`, `{% for %}` engine
- **Colored logging** — built-in `Logger` with per-level ANSI colors
- **Cookie support** — full cookie jar on request and response
- **Keep-alive** — persistent connections handled automatically

---

## Installation
```toml
[dependencies]
sunweb = "0.3.0"
```

With templating:
```toml
[dependencies]
sunweb = { version = "0.3.0", features = ["templating"] }
```

---

## Quick Start
```rust
use sunweb::{App, get, HTTPRequest, Response};
use log::LevelFilter;

#[derive(App)]
struct MyApp;

#[get("/")]
async fn index(req: HTTPRequest) -> Response {
    Response::ok()
        .set_html()
        .set_body_string("Hello from SunWeb!".into())
}

fn main() {
    sunweb::Logger::init(LevelFilter::Info);

    MyApp::builder()
        .http("0.0.0.0:8080")
        .run();
}
```

---

## HTTPS
```rust
MyApp::builder()
    .http("0.0.0.0:8080")       // redirects to HTTPS automatically
    .https("0.0.0.0:8443")
    .cert("key.pem", "cert.pem")
    .domain("example.com")
    .run();
```

---

## Routing
```rust
use sunweb::{get, post, delete, HTTPRequest, Response, JsonResponse, TextResponse};

#[get("/users")]
async fn list_users(req: HTTPRequest) -> JsonResponse {
    JsonResponse::ok(r#"{"users": []}"#)
}

#[post("/users")]
async fn create_user(req: HTTPRequest) -> JsonResponse {
    let name = req.form("name").unwrap_or("unknown");
    JsonResponse::ok(format!(r#"{{"name": "{}"}}"#, name))
}

#[delete("/users")]
async fn delete_user(req: HTTPRequest) -> Response {
    Response::no_content()
}
```

---

## Static Files
```rust
use sunweb::static_files;

#[static_files("/assets", "./public")]
struct Assets;
```

---

## Reverse Proxy
```rust
use sunweb::proxy;

#[proxy("/api", "http://localhost:3000")]
struct ApiProxy;
```

---

## Middleware
```rust
use sunweb::{middleware, HTTPRequest, Response};

// Applies to all routes
#[middleware]
fn log_requests(req: &mut HTTPRequest) {
    println!("→ {} {}", req.method(), req.path());
}

// Scoped to /admin/*
#[middleware("/admin")]
fn require_auth(req: &mut HTTPRequest) {
    // reject unauthenticated requests
}

// Response middleware
#[middleware]
fn add_cors(res: &mut Response) {
    res.set_cors_origin("*");
}
```

---

## Error Pages
```rust
use sunweb::{error_page, HTTPRequest, HtmlResponse, TextResponse};

#[error_page(404)]
fn not_found(req: HTTPRequest) -> HtmlResponse {
    HtmlResponse::ok("404 — Page Not Found")
}

#[error_page(500)]
fn server_error(req: HTTPRequest) -> HtmlResponse {
    HtmlResponse::ok("500 — Something went wrong")
}
```

---

## Templating *(feature = "templating")*
```rust
use sunweb::{get, render, HTTPRequest, Response, Context, Value};

#[get("/")]
async fn index(req: HTTPRequest) -> Response {
    let mut ctx = Context::new();
    ctx.insert("title".into(), Value::from("Home"));
    ctx.insert("logged_in".into(), Value::Bool(true));
    ctx.insert("users".into(), Value::List(vec![
        [("user.name".into(), Value::from("Alice"))].into(),
        [("user.name".into(), Value::from("Bob"))].into(),
    ]));
    render!("index.html", ctx)
}
```

Template syntax:

| Syntax | Description |
|---|---|
| `{{ name }}` | Variable interpolation |
| `{% if condition %}` … `{% endif %}` | Conditional block |
| `{% for item in list %}` … `{% endfor %}` | Loop |

---

## Logging
```rust
use sunweb::Logger;
use log::LevelFilter;

Logger::init(LevelFilter::Info);

log::info!("Server starting");
log::warn!("Something looks off");
log::error!("Something broke");
```

| Level   | Color  |
|---------|--------|
| `error` | Red    |
| `warn`  | Yellow |
| `info`  | Blue   |
| `debug` | Green  |
| `trace` | Dimmed |

---

## Response Types

| Type | Content-Type |
|---|---|
| `HtmlResponse` | `text/html` |
| `JsonResponse` | `application/json` |
| `PlainTextResponse` | `text/plain` |
| `ImageResponse` | `image/*` |
| `RedirectResponse` | — sets `Location` header |
| `NoContentResponse` | `204 No Content` |
| `Response` | fully manual |

---

## Workspace Structure
```
sunweb/
├── sunweb/              # Main crate — public API & re-exports
├── sunweb_core/         # HTTP types, server runtime, request/response
├── sunweb_macros/       # Procedural macros (#[get], #[middleware], etc.)
├── sunweb_templating/   # Optional templating engine
└── example_app/         # Usage examples (not published)
```

---

## License

MIT — see [LICENSE](LICENSE) for details.

## Contributing

Pull requests are welcome.

## Author

[Sunnickel](https://github.com/Sunnickel)

## Links

- [Repository](https://github.com/Sunnickel/SunWeb)
- [Documentation](https://docs.rs/sunweb/latest/sunweb)

## AI Assistance

This project's documentation, including this `README.md` file, has been created with the assistance of AI.

Specifically, AI was used for:

*   Documentation generation
*   Content creation for this `README.md`

All content has been reviewed and edited to ensure accuracy and clarity.
While AI tools were utilized to accelerate the writing process,
all information has been manually checked for correctness.