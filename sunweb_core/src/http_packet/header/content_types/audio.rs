use std::str::FromStr;

/// Represents different audio subtypes.
///
/// This enum covers common audio formats used in web and media
/// applications, including MPEG/MP3, MP4, OGG, WebM, AAC, WAV, and FLAC.
/// Any other audio subtype not explicitly listed is captured by the `Other` variant.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// let a = AudioSubType::from_str("mp3").unwrap();
/// assert_eq!(a, AudioSubType::Mpeg);
///
/// let unknown = AudioSubType::from_str("opus").unwrap();
/// assert_eq!(unknown, AudioSubType::Other("opus".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AudioSubType {
    /// MPEG audio (MP3)
    Mpeg,
    /// MP4 audio
    Mp4,
    /// OGG audio
    Ogg,
    /// WebM audio
    Webm,
    /// AAC audio
    Aac,
    /// WAV audio
    Wav,
    /// FLAC audio
    Flac,
    /// Any other audio subtype not listed above
    Other(String),
}

impl FromStr for AudioSubType {
    type Err = ();

    /// Parses a string into an `AudioSubType`.
    ///
    /// Recognizes `"mpeg"`, `"mp3"`, `"mp4"`, `"ogg"`, `"webm"`, `"aac"`, `"wav"`, and `"flac"`.
    /// Any other string will be wrapped in `AudioSubType::Other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr;
    /// use your_crate::AudioSubType;
    ///
    /// let audio = AudioSubType::from_str("flac").unwrap();
    /// assert_eq!(audio, AudioSubType::Flac);
    /// ```
    fn from_str(sub: &str) -> Result<Self, Self::Err> {
        Ok(match sub {
            "mpeg" | "mp3" => AudioSubType::Mpeg,
            "mp4" => AudioSubType::Mp4,
            "ogg" => AudioSubType::Ogg,
            "webm" => AudioSubType::Webm,
            "aac" => AudioSubType::Aac,
            "wav" => AudioSubType::Wav,
            "flac" => AudioSubType::Flac,
            other => AudioSubType::Other(other.into()),
        })
    }
}
