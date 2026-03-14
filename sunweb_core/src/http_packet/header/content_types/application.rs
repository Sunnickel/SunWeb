//! Registered and common `application/*` sub-types.
//!
//! Unknown sub-types are preserved via [`Other(String)`](ApplicationSubType::Other)
//! so no information is lost during round-trips.

use std::str::FromStr;

/// Sub-type portion of an `application/*` MIME type.
///
/// ```
/// use http_packet::header::content_types::application::ApplicationSubType;
/// use std::str::FromStr;
///
/// let json = ApplicationSubType::from_str("json").unwrap();
/// assert_eq!(json.to_string(), "json");
///
/// let custom = ApplicationSubType::from_str("vnd.api+json").unwrap();
/// assert_eq!(custom.to_string(), "vnd.api+json");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApplicationSubType {
    /// `application/json`
    Json,
    /// `application/xml`
    Xml,
    /// `application/octet-stream`
    OctetStream,
    /// `application/pdf`
    Pdf,
    /// `application/zip`
    Zip,
    /// `application/gzip`
    Gzip,
    /// `application/x-www-form-urlencoded`
    XWwwFormUrlEncoded,
    /// `application/wasm`
    Wasm,
    /// `application/javascript`
    Javascript,
    /// Anything else; stored verbatim.
    Other(String),
}

impl FromStr for ApplicationSubType {
    type Err = ();

    fn from_str(sub: &str) -> Result<Self, Self::Err> {
        Ok(match sub {
            "json" => Self::Json,
            "xml" => Self::Xml,
            "octet-stream" => Self::OctetStream,
            "pdf" => Self::Pdf,
            "zip" => Self::Zip,
            "gzip" => Self::Gzip,
            "x-www-form-urlencoded" => Self::XWwwFormUrlEncoded,
            "wasm" => Self::Wasm,
            "javascript" | "js" => Self::Javascript,
            other => Self::Other(other.into()),
        })
    }
}
