use crate::http_packet::header::content_types::ContentType;
use std::str::FromStr;
use std::{
    fs,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    sync::Arc,
};

/// Retrieves the content and MIME type of a static file based on a route and base folder.
///
/// This function maps a given route to a file path relative to a specified folder,
/// reads the file's contents, and infers its MIME type from the file extension.
/// If the file does not exist, it returns an empty string with `text/plain` as the MIME type.
///
/// # Arguments
///
/// * `route` - The route path to the file, e.g., `/static/css/style.css`.
/// * `folder` - The base folder where static files are located.
///
/// # Returns
///
/// A tuple containing:
/// * `Arc<String>` — the file's content.
/// * `String` — the inferred MIME type of the file.
///
/// # MIME Type Mapping
///
/// | Extension | MIME Type                  |
/// |-----------|----------------------------|
/// | css       | text/css                   |
/// | js        | application/javascript     |
/// | html      | text/html                  |
/// | json      | application/json           |
/// | png       | image/png                  |
/// | jpg/jpeg  | image/jpeg                 |
/// | svg       | image/svg+xml              |
/// | other     | text/plain                 |
///
/// # Examples
///
/// ```rust
/// use std::fs;
/// use std::sync::Arc;
/// use tempfile::tempdir;
/// use crate::server::files::get_static_file_content;
///
/// let dir = tempdir().unwrap();
/// let folder = dir.path().to_str().unwrap().to_string();
/// let file_path = format!("{}/style.css", folder);
/// fs::write(&file_path, "body { color: red; }").unwrap();
///
/// let (content, mime_type) = get_static_file_content("/static/css/style.css", &folder);
///
/// assert_eq!(mime_type, "text/css");
/// assert!(content.contains("color: red"));
/// ```
pub(crate) fn get_static_file_content(route: &str, folder: &String) -> (Arc<String>, ContentType) {
    let parts: Vec<&str> = route.trim_start_matches('/').splitn(2, '/').collect();
    let relative_path = if parts.len() > 1 { parts[1] } else { "" };
    let file_path = Path::new(folder).join(relative_path);

    log::debug!("Resolved static path: {}", file_path.display());

    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("html") => "text/html",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => "text/plain",
    };

    match fs::read_to_string(&file_path) {
        Ok(content) => (
            Arc::new(content),
            ContentType::from_str(content_type).expect("Could not parse ContentType!"),
        ),
        Err(e) => {
            log::warn!("Static file not found: {} ({})", file_path.display(), e);
            (
                Arc::new(String::new()),
                ContentType::from_str("text/plain").expect("Could not parse ContentType!"),
            )
        }
    }
}

/// Reads the entire content of a file into an `Arc<String>`.
///
/// This function opens the specified file, reads its contents into a string,
/// and returns it wrapped in an `Arc`. It will panic if the file cannot be opened
/// or read. If the file does not exist, it returns an empty string.
///
/// # Arguments
///
/// * `file_path` - The path to the file to read.
///
/// # Returns
///
/// An `Arc<String>` containing the full file contents.
///
/// # Panics
///
/// Panics if the file exists but cannot be opened or read successfully.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
/// use std::sync::Arc;
/// use crate::server::files::get_file_content;
///
/// let content: Arc<String> = get_file_content(&Path::new("example.txt"));
/// ```
pub fn get_file_content(file_path: &Path) -> Arc<String> {
    let file =
        File::open(file_path).unwrap_or_else(|_| panic!("Cannot open {}", file_path.display()));
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader
        .read_to_string(&mut contents)
        .expect("File couldn't be read");
    Arc::new(contents)
}
