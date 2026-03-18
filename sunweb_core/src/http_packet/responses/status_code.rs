//! # HTTP Status Codes Module
//!
//! This module provides a comprehensive enumeration of HTTP status codes organized by category.
//! It implements standard HTTP/1.1 status codes as defined in RFC 7231 and related specifications.
//!
//! # Categories
//!
//! - **1xx Informational**: Request received, continuing process
//! - **2xx Success**: Request successfully received, understood, and accepted
//! - **3xx Redirection**: Further action needs to be taken to complete the request
//! - **4xx Client Error**: Request contains bad syntax or cannot be fulfilled
//! - **5xx Server Error**: Server failed to fulfill an apparently valid request
//!
//! # Examples
//!
//! ```rust
//! use sunweb::StatusCode;
//!
//! // Create a status code
//! let status = StatusCode::Ok;
//! assert_eq!(status.as_u16(), 200);
//! assert_eq!(status.to_string(), "OK");
//!
//! // Compare status codes
//! let not_found = StatusCode::NotFound;
//! assert_eq!(not_found.as_u16(), 404);
//! assert!(!status.equals(not_found));
//!
//! // Use in pattern matching
//! match status {
//!     StatusCode::Ok => println!("Success!"),
//!     StatusCode::NotFound => println!("Not found"),
//!     _ => println!("Other status"),
//! }
//! ```

use std::fmt;
use std::fmt::Formatter;

/// HTTP response status codes enumeration
///
/// Represents all standard HTTP response status codes organized by class.
/// Each variant is explicitly assigned its corresponding numeric value to match
/// the HTTP status code standard defined in RFC 7231 and related RFCs.
///
/// The enum uses `#[repr(u16)]` to ensure each variant corresponds to its
/// standard HTTP status code number.
///
/// # Supported Status Codes
///
/// ## 1xx Informational
/// - `Continue` (100): Initial part of request received, client should continue
/// - `SwitchingProtocols` (101): Server switching protocols per client request
/// - `Processing` (102): WebDAV - request being processed
/// - `EarlyHints` (103): Used to return response headers before final response
///
/// ## 2xx Success
/// - `Ok` (200): Standard response for successful HTTP requests
/// - `Created` (201): Request fulfilled, new resource created
/// - `Accepted` (202): Request accepted for processing
/// - `NoContent` (204): Success with no content to return
/// - And more...
///
/// ## 3xx Redirection
/// - `MovedPermanently` (301): Resource permanently moved
/// - `Found` (302): Resource temporarily moved
/// - `TemporaryRedirect` (307): Temporary redirect maintaining method
/// - And more...
///
/// ## 4xx Client Error
/// - `BadRequest` (400): Server cannot process due to client error
/// - `Unauthorized` (401): Authentication required
/// - `Forbidden` (403): Server refuses to authorize
/// - `NotFound` (404): Resource not found
/// - And more...
///
/// ## 5xx Server Error
/// - `InternalServerError` (500): Generic server error
/// - `BadGateway` (502): Invalid response from upstream server
/// - `ServiceUnavailable` (503): Server temporarily unavailable
/// - And more...
///
/// # Examples
///
/// ```rust
/// use sunweb::StatusCode;
///
/// let status = StatusCode::Ok;
/// assert_eq!(status as u16, 200);
///
/// let error = StatusCode::InternalServerError;
/// assert_eq!(error.as_u16(), 500);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u16)]
pub enum StatusCode {
    // 1xx Informational
    /// 100 Continue - Initial part of request received, client should continue
    Continue = 100,
    /// 101 Switching Protocols - Server switching protocols per Upgrade header
    SwitchingProtocols = 101,
    /// 102 Processing - WebDAV; request received but not yet completed
    Processing = 102,
    /// 103 Early Hints - Used to return some response headers before final response
    EarlyHints = 103,

    // 2xx Success
    /// 200 OK - Standard response for successful HTTP requests
    Ok = 200,
    /// 201 Created - Request fulfilled, new resource created
    Created = 201,
    /// 202 Accepted - Request accepted for processing, but not completed
    Accepted = 202,
    /// 203 Non-Authoritative Information - Successful but transformed response
    NonAuthoritativeInformation = 203,
    /// 204 No Content - Successful request with no content to return
    NoContent = 204,
    /// 205 Reset Content - Server fulfilled request, user agent should reset document view
    ResetContent = 205,
    /// 206 Partial Content - Server delivering only part of resource due to range header
    PartialContent = 206,
    /// 207 Multi-Status - WebDAV; multiple status codes might be appropriate
    MultiStatus = 207,
    /// 208 Already Reported - WebDAV; members already enumerated
    AlreadyReported = 208,
    /// 226 IM Used - Server fulfilled GET request with instance-manipulations
    ImUsed = 226,

    // 3xx Redirection
    /// 300 Multiple Choices - Multiple options for the resource
    MultipleChoices = 300,
    /// 301 Moved Permanently - Resource permanently moved to new URI
    MovedPermanently = 301,
    /// 302 Found - Resource temporarily under different URI
    Found = 302,
    /// 303 See Other - Response to request found under different URI using GET
    SeeOther = 303,
    /// 304 Not Modified - Resource not modified since last request
    NotModified = 304,
    /// 307 Temporary Redirect - Resource temporarily under different URI, maintain method
    TemporaryRedirect = 307,
    /// 308 Permanent Redirect - Resource permanently moved, maintain method
    PermanentRedirect = 308,

    // 4xx Client Error
    /// 400 Bad Request - Server cannot process due to client error
    BadRequest = 400,
    /// 401 Unauthorized - Authentication required
    Unauthorized = 401,
    /// 402 Payment Required - Reserved for future use
    PaymentRequired = 402,
    /// 403 Forbidden - Server refuses to authorize request
    Forbidden = 403,
    /// 404 Not Found - Requested resource not found
    NotFound = 404,
    /// 405 Method Not Allowed - Request method not supported for resource
    MethodNotAllowed = 405,
    /// 406 Not Acceptable - Resource not available matching Accept headers
    NotAcceptable = 406,
    /// 407 Proxy Authentication Required - Client must authenticate with proxy
    ProxyAuthenticationRequired = 407,
    /// 408 Request Timeout - Server timed out waiting for request
    RequestTimeout = 408,
    /// 409 Conflict - Request conflicts with current state
    Conflict = 409,
    /// 410 Gone - Resource no longer available
    Gone = 410,
    /// 411 Length Required - Content-Length header required
    LengthRequired = 411,
    /// 412 Precondition Failed - Preconditions in headers not met
    PreconditionFailed = 412,
    /// 413 Content Too Large - Request entity larger than server limits
    ContentTooLarge = 413,
    /// 414 URI Too Long - URI longer than server can process
    UriTooLong = 414,
    /// 415 Unsupported Media Type - Media type not supported
    UnsupportedMediaType = 415,
    /// 416 Range Not Satisfiable - Range header cannot be satisfied
    RangeNotSatisfiable = 416,
    /// 417 Expectation Failed - Expect header requirement cannot be met
    ExpectationFailed = 417,
    /// 418 I'm a teapot - RFC 2324 April Fools joke
    ImATeapot = 418,
    /// 421 Misdirected Request - Request directed at wrong server
    MisdirectedRequest = 421,
    /// 422 Unprocessable Content - WebDAV; semantically erroneous request
    UnprocessableContent = 422,
    /// 423 Locked - WebDAV; resource locked
    Locked = 423,
    /// 424 Failed Dependency - WebDAV; request failed due to previous failure
    FailedDependency = 424,
    /// 425 Too Early - Server unwilling to risk processing replayed request
    TooEarly = 425,
    /// 426 Upgrade Required - Client should switch to different protocol
    UpgradeRequired = 426,
    /// 428 Precondition Required - Origin server requires conditional request
    PreconditionRequired = 428,
    /// 429 Too Many Requests - User sent too many requests in given time
    TooManyRequests = 429,
    /// 431 Request Header Fields Too Large - Headers too large
    RequestHeaderFieldsTooLarge = 431,
    /// 451 Unavailable For Legal Reasons - Resource unavailable for legal reasons
    UnavailableForLegalReasons = 451,

    // 5xx Server Error
    /// 500 Internal Server Error - Generic server error
    InternalServerError = 500,
    /// 501 Not Implemented - Server doesn't support functionality
    NotImplemented = 501,
    /// 502 Bad Gateway - Invalid response from upstream server
    BadGateway = 502,
    /// 503 Service Unavailable - Server temporarily unavailable
    ServiceUnavailable = 503,
    /// 504 Gateway Timeout - Upstream server timeout
    GatewayTimeout = 504,
    /// 505 HTTP Version Not Supported - HTTP version not supported
    HTTPVersionNotSupported = 505,
    /// 506 Variant Also Negotiates - Transparent content negotiation error
    VariantAlsoNegotiates = 506,
    /// 507 Insufficient Storage - WebDAV; server cannot store representation
    InsufficientStorage = 507,
    /// 508 Loop Detected - WebDAV; infinite loop detected
    LoopDetected = 508,
    /// 509 Not Extended - Further extensions required
    NotExtended = 509,
    /// 510 Network Authentication Required - Client must authenticate for network access
    NetworkAuthenticationRequired = 510,
}

impl From<u16> for StatusCode {
    fn from(code: u16) -> Self {
        match code {
            100 => StatusCode::Continue,
            101 => StatusCode::SwitchingProtocols,
            102 => StatusCode::Processing,
            103 => StatusCode::EarlyHints,

            200 => StatusCode::Ok,
            201 => StatusCode::Created,
            202 => StatusCode::Accepted,
            203 => StatusCode::NonAuthoritativeInformation,
            204 => StatusCode::NoContent,
            205 => StatusCode::ResetContent,
            206 => StatusCode::PartialContent,
            207 => StatusCode::MultiStatus,
            208 => StatusCode::AlreadyReported,
            226 => StatusCode::ImUsed,

            300 => StatusCode::MultipleChoices,
            301 => StatusCode::MovedPermanently,
            302 => StatusCode::Found,
            303 => StatusCode::SeeOther,
            304 => StatusCode::NotModified,
            307 => StatusCode::TemporaryRedirect,
            308 => StatusCode::PermanentRedirect,

            400 => StatusCode::BadRequest,
            401 => StatusCode::Unauthorized,
            402 => StatusCode::PaymentRequired,
            403 => StatusCode::Forbidden,
            404 => StatusCode::NotFound,
            405 => StatusCode::MethodNotAllowed,
            406 => StatusCode::NotAcceptable,
            407 => StatusCode::ProxyAuthenticationRequired,
            408 => StatusCode::RequestTimeout,
            409 => StatusCode::Conflict,
            410 => StatusCode::Gone,
            411 => StatusCode::LengthRequired,
            412 => StatusCode::PreconditionFailed,
            413 => StatusCode::ContentTooLarge,
            414 => StatusCode::UriTooLong,
            415 => StatusCode::UnsupportedMediaType,
            416 => StatusCode::RangeNotSatisfiable,
            417 => StatusCode::ExpectationFailed,
            418 => StatusCode::ImATeapot,
            421 => StatusCode::MisdirectedRequest,
            422 => StatusCode::UnprocessableContent,
            423 => StatusCode::Locked,
            424 => StatusCode::FailedDependency,
            425 => StatusCode::TooEarly,
            426 => StatusCode::UpgradeRequired,
            428 => StatusCode::PreconditionRequired,
            429 => StatusCode::TooManyRequests,
            431 => StatusCode::RequestHeaderFieldsTooLarge,
            451 => StatusCode::UnavailableForLegalReasons,

            501 => StatusCode::NotImplemented,
            502 => StatusCode::BadGateway,
            503 => StatusCode::ServiceUnavailable,
            504 => StatusCode::GatewayTimeout,
            505 => StatusCode::HTTPVersionNotSupported,
            506 => StatusCode::VariantAlsoNegotiates,
            507 => StatusCode::InsufficientStorage,
            508 => StatusCode::LoopDetected,
            509 => StatusCode::NotExtended,
            510 => StatusCode::NetworkAuthenticationRequired,

            _ => StatusCode::InternalServerError,
        }
    }
}

impl fmt::Display for StatusCode {
    /// Formats the status code as its standard reason phrase
    ///
    /// Returns the human-readable reason phrase associated with each status code
    /// as defined in the HTTP specifications.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sunweb::StatusCode;
    ///
    /// assert_eq!(StatusCode::Ok.to_string(), "OK");
    /// assert_eq!(StatusCode::NotFound.to_string(), "Not Found");
    /// assert_eq!(StatusCode::InternalServerError.to_string(), "Internal Server Error");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                // 1xx
                StatusCode::Continue => "Continue",
                StatusCode::SwitchingProtocols => "Switching Protocols",
                StatusCode::Processing => "Processing",
                StatusCode::EarlyHints => "Early Hints",

                // 2xx
                StatusCode::Ok => "OK",
                StatusCode::Created => "Created",
                StatusCode::Accepted => "Accepted",
                StatusCode::NonAuthoritativeInformation => "Non-Authoritative Information",
                StatusCode::NoContent => "No Content",
                StatusCode::ResetContent => "Reset Content",
                StatusCode::PartialContent => "Partial Content",
                StatusCode::MultiStatus => "Multi-Status",
                StatusCode::AlreadyReported => "Already Reported",
                StatusCode::ImUsed => "IM Used",

                // 3xx
                StatusCode::MultipleChoices => "Multiple Choices",
                StatusCode::MovedPermanently => "Moved Permanently",
                StatusCode::Found => "Found",
                StatusCode::SeeOther => "See Other",
                StatusCode::NotModified => "Not Modified",
                StatusCode::TemporaryRedirect => "Temporary Redirect",
                StatusCode::PermanentRedirect => "Permanent Redirect",

                // 4xx
                StatusCode::BadRequest => "Bad Request",
                StatusCode::Unauthorized => "Unauthorized",
                StatusCode::PaymentRequired => "Payment Required",
                StatusCode::Forbidden => "Forbidden",
                StatusCode::NotFound => "Not Found",
                StatusCode::MethodNotAllowed => "Method Not Allowed",
                StatusCode::NotAcceptable => "Not Acceptable",
                StatusCode::ProxyAuthenticationRequired => "Proxy Authentication Required",
                StatusCode::RequestTimeout => "Request Timeout",
                StatusCode::Conflict => "Conflict",
                StatusCode::Gone => "Gone",
                StatusCode::LengthRequired => "Length Required",
                StatusCode::PreconditionFailed => "Precondition Failed",
                StatusCode::ContentTooLarge => "Content Too Large",
                StatusCode::UriTooLong => "URI Too Long",
                StatusCode::UnsupportedMediaType => "Unsupported Media Type",
                StatusCode::RangeNotSatisfiable => "Range Not Satisfiable",
                StatusCode::ExpectationFailed => "Expectation Failed",
                StatusCode::ImATeapot => "I'm a teapot",
                StatusCode::MisdirectedRequest => "Misdirected Request",
                StatusCode::UnprocessableContent => "Unprocessable Content",
                StatusCode::Locked => "Locked",
                StatusCode::FailedDependency => "Failed Dependency",
                StatusCode::TooEarly => "Too Early",
                StatusCode::UpgradeRequired => "Upgrade Required",
                StatusCode::PreconditionRequired => "Precondition Required",
                StatusCode::TooManyRequests => "Too Many Requests",
                StatusCode::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
                StatusCode::UnavailableForLegalReasons => "Unavailable For Legal Reasons",

                // 5xx
                StatusCode::InternalServerError => "Internal Server Error",
                StatusCode::NotImplemented => "Not Implemented",
                StatusCode::BadGateway => "Bad Gateway",
                StatusCode::ServiceUnavailable => "Service Unavailable",
                StatusCode::GatewayTimeout => "Gateway Timeout",
                StatusCode::HTTPVersionNotSupported => "HTTP Version Not Supported",
                StatusCode::VariantAlsoNegotiates => "Variant Also Negotiates",
                StatusCode::InsufficientStorage => "Insufficient Storage",
                StatusCode::LoopDetected => "Loop Detected",
                StatusCode::NotExtended => "Not Extended",
                StatusCode::NetworkAuthenticationRequired => "Network Authentication Required",
            }
        )
    }
}

impl StatusCode {
    /// Compares two status codes for complete equality
    ///
    /// This method checks both the numeric value and string representation of status codes
    /// for equality. This is more thorough than the `PartialEq` implementation which only
    /// compares the enum variants.
    ///
    /// # Arguments
    ///
    /// * `response_codes` - The other status code to compare against
    ///
    /// # Returns
    ///
    /// Returns `true` if both the numeric value and string representation match,
    /// `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sunweb::StatusCode;
    ///
    /// let status1 = StatusCode::Ok;
    /// let status2 = StatusCode::Ok;
    /// assert!(status1.equals(status2));
    ///
    /// let status3 = StatusCode::NotFound;
    /// assert!(!status1.equals(status3));
    /// ```
    pub fn equals(&self, response_codes: StatusCode) -> bool {
        self.as_u16() == response_codes.as_u16() && self.to_string() == response_codes.to_string()
    }

    /// Gets the numeric value of the status code
    ///
    /// Returns the HTTP status code as a `u16` integer value, matching the standard
    /// HTTP status code numbers (e.g., 200, 404, 500).
    ///
    /// # Returns
    ///
    /// The u16 representation of this HTTP status code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sunweb::StatusCode;
    ///
    /// assert_eq!(StatusCode::Ok.as_u16(), 200);
    /// assert_eq!(StatusCode::NotFound.as_u16(), 404);
    /// assert_eq!(StatusCode::InternalServerError.as_u16(), 500);
    /// assert_eq!(StatusCode::Continue.as_u16(), 100);
    /// ```
    pub fn as_u16(&self) -> u16 {
        *self as u16
    }
}
