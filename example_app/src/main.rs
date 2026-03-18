use std::collections::HashMap;
use std::path::Path;
use sunweb::http_packet::responses::status_code::StatusCode;
use sunweb::*;

#[derive(App)]
struct MainApp;

#[middleware]
fn cors(req: &mut HTTPRequest, res: &mut Response) {
    if let Some(origin) = req.header("Origin") {
        res.set_cors_origin(&origin);

        res.set_cors_methods(&["GET", "POST", "PUT", "DELETE", "OPTIONS"]);
        res.set_cors_headers(&["Content-Type", "Authorization"]);
        res.set_cors_max_age(3600);
        res.set_cors_credentials(true);
    }
}

#[middleware("/api")]
fn api_cors(req: &mut HTTPRequest, res: &mut Response) {
    res.apply_cors_permissive();
    log::info!("API cors applied for: {}", req.path());
}

#[static_files("/static", "./example_app/resources/static")]
struct PublicFiles;

#[get("/")]
fn index(_: &HTTPRequest) -> HtmlResponse {
    let content = get_file_content(Path::new("./example_app/resources/templates/index.html"));
    HtmlResponse::ok(content.as_str())
}

#[get("/template")]
fn template(_: &HTTPRequest) -> HtmlResponse {
    let content = get_file_content(Path::new(
        "./example_app/resources/templates/template_test.html",
    ));
    let mut ctx: Context = HashMap::new();

    // ── Variables ──────────────────────────────────────────────────────────
    ctx.insert("page_title".into(), Value::from("SunWeb Template Tester"));
    ctx.insert("username".into(), Value::from("alice smith"));
    ctx.insert("score".into(), Value::Num(42.0));
    ctx.insert("negative".into(), Value::Num(-7.0));
    ctx.insert("pi".into(), Value::Num(std::f64::consts::PI));
    ctx.insert("empty_var".into(), Value::from(""));
    ctx.insert("safe_html".into(), Value::from("<strong>bold</strong>"));
    ctx.insert(
        "unsafe_html".into(),
        Value::from("<script>alert('xss')</script>"),
    );

    // ── If conditions ──────────────────────────────────────────────────────
    ctx.insert("logged_in".into(), Value::Bool(true));
    ctx.insert("is_admin".into(), Value::Bool(false));
    ctx.insert("is_moderator".into(), Value::Bool(true));
    ctx.insert("level".into(), Value::Num(5.0));

    // ── For loop ───────────────────────────────────────────────────────────
    ctx.insert(
        "users".into(),
        Value::List(vec![
            [
                ("name".into(), Value::from("Alice")),
                ("role".into(), Value::from("Admin")),
                ("active".into(), Value::Bool(true)),
            ]
            .into(),
            [
                ("name".into(), Value::from("Bob")),
                ("role".into(), Value::from("Editor")),
                ("active".into(), Value::Bool(false)),
            ]
            .into(),
            [
                ("name".into(), Value::from("Carol")),
                ("role".into(), Value::from("Viewer")),
                ("active".into(), Value::Bool(true)),
            ]
            .into(),
        ]),
    );

    // Empty list — triggers {% else %} on for
    ctx.insert("notifications".into(), Value::List(vec![]));

    render!(content, ctx)
}

#[get("/hello")]
fn hello(_: &HTTPRequest) -> JsonResponse {
    JsonResponse::ok(r#"{"message": "Hello!"}"#)
}

#[get("/about")]
fn about(_: &HTTPRequest) -> HtmlResponse {
    HtmlResponse::ok("<h1>About us</h1>")
}

#[get("/text")]
fn text(_: &HTTPRequest) -> PlainTextResponse {
    PlainTextResponse::ok("Hello plain!")
}

#[get("/old-about")]
fn old_about(_: &HTTPRequest) -> RedirectResponse {
    RedirectResponse::permanent("/about")
}

#[get("/no-content")]
fn no_content(_: &HTTPRequest) -> NoContentResponse {
    NoContentResponse
}

#[get("/api/test")]
fn api_test(_: &HTTPRequest) -> JsonResponse {
    JsonResponse::ok(r#"{"status": "ok"}"#)
}

#[error_page(404)]
fn not_found(_: &HTTPRequest) -> HtmlResponse {
    HtmlResponse::status("<h1>Not found</h1>", StatusCode::NotFound)
}

#[error_page(500)]
fn server_error(_: &HTTPRequest) -> HtmlResponse {
    HtmlResponse::status(
        "<h1>Something went wrong</h1>",
        StatusCode::InternalServerError,
    )
}

fn main() {
    Logger::init(log::LevelFilter::Info);

    MainApp::builder()
        .http("0.0.0.0:80")
        .https("0.0.0.0:443")
        .cert(
            "./example_app/resources/cert/key.pem",
            "./example_app/resources/cert/cert.pem",
        )
        .run();
}
