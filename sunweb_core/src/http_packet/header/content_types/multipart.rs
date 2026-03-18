//! Registered and common `multipart/*` sub-types.
//!
//! Unknown sub-types are preserved via [`Other(String)`](MultipartSubType::Other)
//! so no information is lost during round-trips.

use std::str::FromStr;

/// Sub-type portion of a `multipart/*` MIME type.
///
/// ```
/// use http_packet::header::content_types::multipart::MultipartSubType;
/// use std::str::FromStr;
///
/// let form = MultipartSubType::from_str("form-data").unwrap();
/// assert_eq!(form.to_string(), "form-data");
///
/// let custom = MultipartSubType::from_str("byteranges").unwrap();
/// assert_eq!(custom.to_string(), "byteranges");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MultipartSubType {
    /// `multipart/form-data`
    FormData,
    /// `multipart/mixed`
    Mixed,
    /// `multipart/alternative`
    Alternative,
    /// `multipart/related`
    Related,
    /// Anything else; stored verbatim.
    Other(String),
}

impl FromStr for MultipartSubType {
    type Err = ();

    fn from_str(sub: &str) -> Result<Self, Self::Err> {
        Ok(match sub {
            "form-data" => Self::FormData,
            "mixed" => Self::Mixed,
            "alternative" => Self::Alternative,
            "related" => Self::Related,
            other => Self::Other(other.into()),
        })
    }
}
