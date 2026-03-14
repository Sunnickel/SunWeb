//! Registered and common `text/*` sub-types.
//!
//! Unknown sub-types are preserved via [`Other(String)`](TextSubType::Other)
//! so no information is lost during round-trips.

use std::str::FromStr;

/// Sub-type portion of a `text/*` MIME type.
///
/// ```
/// use http_packet::header::content_types::text::TextSubType;
/// use std::str::FromStr;
///
/// let html = TextSubType::from_str("html").unwrap();
/// assert_eq!(html.to_string(), "html");
///
/// let custom = TextSubType::from_str("vnd.custom").unwrap();
/// assert_eq!(custom.to_string(), "vnd.custom");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextSubType {
    /// `text/plain`
    Plain,
    /// `text/html`
    Html,
    /// `text/css`
    Css,
    /// `text/javascript` (also accepts `js`)
    Javascript,
    /// `text/csv`
    Csv,
    /// `text/xml`
    Xml,
    /// `text/markdown`
    Markdown,
    /// Anything else; stored verbatim.
    Other(String),
}

impl FromStr for TextSubType {
    type Err = ();

    fn from_str(sub: &str) -> Result<Self, Self::Err> {
        Ok(match sub {
            "plain" => Self::Plain,
            "html" => Self::Html,
            "css" => Self::Css,
            "javascript" | "js" => Self::Javascript,
            "csv" => Self::Csv,
            "xml" => Self::Xml,
            "markdown" => Self::Markdown,
            other => Self::Other(other.into()),
        })
    }
}
