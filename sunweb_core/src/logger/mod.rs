use chrono::Utc;
use log::{Level, Metadata, Record};
use crate::http_packet::requests::HTTPRequest;
use crate::http_packet::responses::Response;

/// ANSI color code for red text.
const RED: &str = "\x1b[31m";

/// ANSI color code for yellow text.
const YELLOW: &str = "\x1b[33m";

/// ANSI color code for blue text.
const BLUE: &str = "\x1b[34m";

/// ANSI color code for green text.
const GREEN: &str = "\x1b[32m";

/// ANSI color code for dimmed text.
const DIM: &str = "\x1b[2m";

/// ANSI color code to reset text formatting.
const RESET: &str = "\x1b[0m";

/// A custom logger that provides colored console output based on log level.
///
/// This logger uses ANSI escape codes to colorize log messages:
/// - `Error` messages are displayed in red.
/// - `Warn` messages are displayed in yellow.
/// - `Info` messages are displayed in blue.
/// - `Debug` messages are displayed in green.
/// - `Trace` messages are displayed dimmed.
///
/// The logger implements the `log::Log` trait, allowing integration with
/// the standard Rust `log` facade.
///
/// # Examples
///
/// ```rust
/// use log::SetLoggerError;
/// use crate::server::logger::Logger;
///
/// # fn main() -> Result<(), SetLoggerError> {
/// log::set_logger(&Logger).unwrap();
/// log::set_max_level(log::LevelFilter::Trace);
/// log::error!("This will appear in red");
/// log::info!("This will appear in blue");
/// # Ok(())
/// # }
/// ```
pub struct Logger;

impl log::Log for Logger {
    /// Determines if a log message should be processed based on its metadata.
    ///
    /// Returns `true` if the log level of the metadata is less than or equal to
    /// the maximum allowed log level set by `log::max_level()`.
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    /// Logs a record with appropriate coloring based on its level.
    ///
    /// Messages are printed directly to stdout using ANSI color codes.
    ///
    /// # Arguments
    ///
    /// * `record` - The log record to process and display.
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

    /// Flushes any buffered records.
    ///
    /// This implementation does nothing because logging writes directly to stdout.
    fn flush(&self) {}
}

impl Logger {
    /// Logs the end of an HTTP request, including the response status code.
    ///
    /// Colors the status code based on the HTTP response class:
    /// - `2xx` is green
    /// - `3xx` is yellow
    /// - `4xx` and `5xx` are red
    /// - Others use the default color
    ///
    /// # Arguments
    ///
    /// * `response` - The HTTP response to log.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::server::logger::Logger;
    /// use crate::server::responses::HTTPResponse;
    ///
    /// let mut res = HTTPResponse::new(200);
    /// Logger::log_request_end(&mut res);
    /// ```
    pub(crate) fn log_request_end(response: &mut Response) {
        let color = match response.status_code.as_u16() {
            200..=299 => GREEN,
            300..=399 => YELLOW,
            400..=599 => RED,
            _ => RESET,
        };

        println!(" {}-> {}{}{}", color, response.status_code, RESET, "");
    }

    pub(crate) fn log_request(request: &mut HTTPRequest, response: &mut Response) {
        let host = request.host().map(|h| h.to_string()).unwrap_or_default();

        let color = match response.status_code.as_u16() {
            200..=299 => GREEN,
            300..=399 => YELLOW,
            400..=599 => RED,
            _ => RESET,
        };

        println!(
            "{}[INFO ]{}[{}] {} [{}] {} {}-> {}{}{}",
            DIM,
            RESET,
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            request.method,
            host,
            request.path,
            color,
            response.status_code,
            RESET,
            ""
        );
    }
}
