//! Media-type registry for HTTP `Content-Type` headers.
//!
//! The root type [`ContentType`] is an enum that covers the seven IANA top-level
//! classes (`text`, `application`, `image`, `audio`, `video`, `font`,
//! `multipart`) plus a fall-through [`Unknown`](ContentType::Unknown) variant
//! for any un-listed combination.
//!
//! Each class has its own submodule containing the currently registered
//! sub-types.  Displaying any value yields the canonical `type/subtype` string
//! that can be placed directly in an HTTP header.
//!
//! # Example
//!
//! ```
//! use http_packet::header::content_types::{ContentType, text::TextSubType};
//!
//! let ct = ContentType::Text(TextSubType::Html);
//! assert_eq!(ct.to_string(), "text/html");
//! ```

pub mod application;
pub mod audio;
pub mod font;
pub mod image;
pub mod multipart;
pub mod text;
pub mod video;

use crate::http_packet::header::content_types::application::ApplicationSubType;
use crate::http_packet::header::content_types::audio::AudioSubType;
use crate::http_packet::header::content_types::font::FontSubType;
use crate::http_packet::header::content_types::image::ImageSubType;
use crate::http_packet::header::content_types::multipart::MultipartSubType;
use crate::http_packet::header::content_types::text::TextSubType;
use crate::http_packet::header::content_types::video::VideoSubType;
use std::fmt;
use std::str::FromStr;

/// A strongly-typed HTTP media type (MIME type).
///
/// Cloning is cheap and every variant is `Eq + Hash`, so values can be used as
/// hash-map keys (e.g. for content negotiation caches).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContentType {
    /// `text/*`
    Text(TextSubType),
    /// `application/*`
    Application(ApplicationSubType),
    /// `image/*`
    Image(ImageSubType),
    /// `audio/*`
    Audio(AudioSubType),
    /// `video/*`
    Video(VideoSubType),
    /// `font/*`
    Font(FontSubType),
    /// `multipart/*`
    Multipart(MultipartSubType),
    /// Fallback for any `type/subtype` combination not explicitly listed above.
    Unknown(String, String),
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (main, sub) = match self {
            ContentType::Text(s) => ("text", s.to_string()),
            ContentType::Application(s) => ("application", s.to_string()),
            ContentType::Image(s) => ("image", s.to_string()),
            ContentType::Audio(s) => ("audio", s.to_string()),
            ContentType::Video(s) => ("video", s.to_string()),
            ContentType::Font(s) => ("font", s.to_string()),
            ContentType::Multipart(s) => ("multipart", s.to_string()),
            ContentType::Unknown(m, s) => (m.as_str(), s.clone()),
        };
        write!(f, "{}/{}", main, sub)
    }
}

impl FromStr for ContentType {
    type Err = ();

    /// Parses a `type/subtype` string into a `ContentType`.
    ///
    /// Any leading or trailing whitespace is **not** trimmed.  If the top-level
    /// type is recognised but the subtype is invalid, the whole parse fails
    /// (`Err(())`).  Unrecognised top-level types fall back to
    /// [`Unknown`](ContentType::Unknown).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (main, sub) = s.split_once('/').unwrap_or(("unknown", "unknown"));
        Ok(match main {
            "text" => ContentType::Text(TextSubType::from_str(sub)?),
            "application" => ContentType::Application(ApplicationSubType::from_str(sub)?),
            "image" => ContentType::Image(ImageSubType::from_str(sub)?),
            "audio" => ContentType::Audio(AudioSubType::from_str(sub)?),
            "video" => ContentType::Video(VideoSubType::from_str(sub)?),
            "font" => ContentType::Font(FontSubType::from_str(sub)?),
            "multipart" => ContentType::Multipart(MultipartSubType::from_str(sub)?),
            other => ContentType::Unknown(other.into(), sub.into()),
        })
    }
}

/// ---------- Display impls for all subtypes ----------
macro_rules! impl_display {
    ($($t:ty),*) => {
        $(
            impl fmt::Display for $t {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    let s = match self {
                        Self::Other(v) => v.clone(),
                        _ => format!("{:?}", self).to_lowercase(),
                    };
                    write!(f, "{}", s.replace('_', "-"))
                }
            }
        )*
    };
}

impl_display!(
    TextSubType,
    ApplicationSubType,
    ImageSubType,
    AudioSubType,
    VideoSubType,
    FontSubType,
    MultipartSubType
);
