use std::path::Path;
use sunweb::http_packet::responses::status_code::StatusCode;
use sunweb::*;

#[derive(App)]
struct MainApp;

#[middleware]
fn cors(res: &mut Response) {
    res.set_cors_origin("http://localhost:80");
    res.set_cors_methods(&["GET", "POST", "PUT", "DELETE", "OPTIONS"]);
    res.set_cors_headers(&["Content-Type", "Authorization"]);
    res.set_cors_max_age(3600);
    res.set_cors_credentials(true);
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

#[get("/slow")]
async fn slow(_: &HTTPRequest) -> PlainTextResponse {
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    PlainTextResponse::ok("done")
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
    MainApp::run("0.0.0.0:8080");
}
