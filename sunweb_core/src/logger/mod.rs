use crate::http_packet::requests::HTTPRequest;
use crate::http_packet::responses::Response;
use chrono::Utc;
use log::{Level, Metadata, Record};

const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const GREEN: &str = "\x1b[32m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

/// Colored console logger for SunWeb.
///
/// Implements [`log::Log`] and can be registered as the global logger via
/// [`Logger::init`]. Once initialized, use the standard [`log`] macros
/// (`error!`, `warn!`, `info!`, `debug!`, `trace!`) anywhere in your app.
///
/// Output is color-coded by level:
///
/// | Level   | Color  |
/// |---------|--------|
/// | `error` | Red    |
/// | `warn`  | Yellow |
/// | `info`  | Blue   |
/// | `debug` | Green  |
/// | `trace` | Dimmed |
///
/// # Example
/// ```rust,ignore
/// use sunweb::Logger;
/// use log::LevelFilter;
///
/// Logger::init(LevelFilter::Info);
///
/// log::info!("Server starting...");
/// log::warn!("Low memory");
/// log::error!("Crashed!");
/// ```
pub struct Logger;

static LOGGER: Logger = Logger;

impl Logger {
    /// Registers `Logger` as the global logger with the given max level.
    ///
    /// Call this once at the start of `main` before starting the server.
    /// Panics if a logger has already been set.
    ///
    /// # Example
    /// ```rust,ignore
    /// use sunweb::Logger;
    /// use log::LevelFilter;
    ///
    /// Logger::init(LevelFilter::Info);
    /// ```
    pub fn init(level: log::LevelFilter) {
        log::set_logger(&LOGGER).expect("Logger already set");
        log::set_max_level(level);
    }

    /// Built-in request/response middleware that prints a colored access log
    /// line for every request.
    ///
    /// Registered automatically by [`WebServer::new`] — you do not need to
    /// call this directly.
    pub(crate) fn log_request(request: &mut HTTPRequest, response: &mut Response) {
        let host = request.host().map(|h| h.to_string()).unwrap_or_default();

        let color = match response.status_code.as_u16() {
            200..=299 => GREEN,
            300..=399 => YELLOW,
            400..=599 => RED,
            _ => RESET,
        };

        println!(
            "{}[INFO ]{}[{}] {} [{}] {} {}-> {}{}",
            DIM,
            RESET,
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            request.method,
            host,
            request.path,
            color,
            response.status_code,
            RESET
        );
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            match record.level() {
                Level::Error => println!("{}[ERROR] - {}{}", RED, record.args(), RESET),
                Level::Trace => println!("{}[TRACE] - {}{}", DIM, record.args(), RESET),
                Level::Warn => println!("{}[WARN ]{} - {}", YELLOW, RESET, record.args()),
                Level::Info => println!("{}[INFO ]{} - {}", BLUE, RESET, record.args()),
                Level::Debug => println!("{}[DEBUG]{} - {}", GREEN, RESET, record.args()),
            }
        }
    }

    /// No-op — output is written directly to stdout with no buffering.
    fn flush(&self) {}
}
