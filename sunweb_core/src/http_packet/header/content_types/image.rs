//! Registered and common `image/*` sub-types.
//!
//! Unknown sub-types are preserved via [`Other(String)`](ImageSubType::Other)
//! so no information is lost during round-trips.

use std::str::FromStr;

/// Sub-type portion of an `image/*` MIME type.
///
/// ```
/// use http_packet::header::content_types::image::ImageSubType;
/// use std::str::FromStr;
///
/// let webp = ImageSubType::from_str("webp").unwrap();
/// assert_eq!(webp.to_string(), "webp");
///
/// let custom = ImageSubType::from_str("vnd.djvu").unwrap();
/// assert_eq!(custom.to_string(), "vnd.djvu");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageSubType {
    /// `image/png`
    Png,
    /// `image/jpeg` (also accepts `jpg`)
    Jpeg,
    /// `image/gif`
    Gif,
    /// `image/webp`
    Webp,
    /// `image/svg+xml`
    SvgXml,
    /// `image/avif`
    Avif,
    /// `image/bmp`
    Bmp,
    /// Anything else; stored verbatim.
    Other(String),
}

impl FromStr for ImageSubType {
    type Err = ();

    fn from_str(sub: &str) -> Result<Self, Self::Err> {
        Ok(match sub {
            "png" => Self::Png,
            "jpeg" | "jpg" => Self::Jpeg,
            "gif" => Self::Gif,
            "webp" => Self::Webp,
            "svg+xml" => Self::SvgXml,
            "avif" => Self::Avif,
            "bmp" => Self::Bmp,
            other => Self::Other(other.into()),
        })
    }
}
