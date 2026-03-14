use std::fmt;
use std::str::FromStr;

/// Represents the standard HTTP methods.
///
/// This enum is used to specify the HTTP method for a request or route.
#[derive(Clone, PartialEq, Debug)]
pub enum HTTPMethod {
    /// GET method.
    GET,
    /// HEAD method.
    HEAD,
    /// OPTIONS method.
    OPTIONS,
    /// TRACE method.
    TRACE,
    /// PUT method.
    PUT,
    /// DELETE method.
    DELETE,
    /// POST method.
    POST,
    /// PATCH method.
    PATCH,
    /// CONNECT method.
    CONNECT,
}

impl FromStr for HTTPMethod {
    type Err = ();

    /// Converts a string to an `HTTPMethod`.
    ///
    /// # Arguments
    ///
    /// * `method` - A string slice representing the HTTP method.
    ///
    /// # Returns
    ///
    /// * `Ok(HTTPMethod)` if the string matches a known method.
    /// * `Err(String)` if the method is unknown.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use your_crate::webserver::route::HTTPMethod;
    ///
    /// let method = HTTPMethod::from_str("POST").unwrap();
    /// assert_eq!(method, HTTPMethod::POST);
    ///
    /// let err = HTTPMethod::from_str("FOO");
    /// assert!(err.is_err());
    /// ```
    fn from_str(method: &str) -> Result<HTTPMethod, ()> {
        match method.to_uppercase().as_str() {
            "GET" => Ok(HTTPMethod::GET),
            "HEAD" => Ok(HTTPMethod::HEAD),
            "OPTIONS" => Ok(HTTPMethod::OPTIONS),
            "TRACE" => Ok(HTTPMethod::TRACE),
            "PUT" => Ok(HTTPMethod::PUT),
            "DELETE" => Ok(HTTPMethod::DELETE),
            "POST" => Ok(HTTPMethod::POST),
            "PATCH" => Ok(HTTPMethod::PATCH),
            "CONNECT" => Ok(HTTPMethod::CONNECT),
            _ => Err(()),
        }
    }
}

impl fmt::Display for HTTPMethod {
    /// Formats the HTTP method as an uppercase string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use your_crate::webserver::route::HTTPMethod;
    ///
    /// let method = HTTPMethod::GET;
    /// assert_eq!(method.to_string(), "GET");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HTTPMethod::GET => write!(f, "GET"),
            HTTPMethod::HEAD => write!(f, "HEAD"),
            HTTPMethod::OPTIONS => write!(f, "OPTIONS"),
            HTTPMethod::TRACE => write!(f, "TRACE"),
            HTTPMethod::PUT => write!(f, "PUT"),
            HTTPMethod::DELETE => write!(f, "DELETE"),
            HTTPMethod::POST => write!(f, "POST"),
            HTTPMethod::PATCH => write!(f, "PATCH"),
            HTTPMethod::CONNECT => write!(f, "CONNECT"),
        }
    }
}
