//! Registered and common `video/*` sub-types.
//!
//! Unknown sub-types are preserved via [`Other(String)`](VideoSubType::Other)
//! so no information is lost during round-trips.

use std::str::FromStr;

/// Sub-type portion of a `video/*` MIME type.
///
/// ```
/// use http_packet::header::content_types::video::VideoSubType;
/// use std::str::FromStr;
///
/// let webm = VideoSubType::from_str("webm").unwrap();
/// assert_eq!(webm.to_string(), "webm");
///
/// let av1 = VideoSubType::from_str("av01").unwrap();
/// assert_eq!(av1.to_string(), "av01");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VideoSubType {
    /// `video/mp4`
    Mp4,
    /// `video/mpeg`
    Mpeg,
    /// `video/webm`
    Webm,
    /// `video/ogg`
    Ogg,
    /// `video/h264`
    H264,
    /// `video/h265`
    H265,
    /// Anything else; stored verbatim.
    Other(String),
}

impl FromStr for VideoSubType {
    type Err = ();

    fn from_str(sub: &str) -> Result<Self, Self::Err> {
        Ok(match sub {
            "mp4" => Self::Mp4,
            "mpeg" => Self::Mpeg,
            "webm" => Self::Webm,
            "ogg" => Self::Ogg,
            "h264" => Self::H264,
            "h265" => Self::H265,
            other => Self::Other(other.into()),
        })
    }
}
