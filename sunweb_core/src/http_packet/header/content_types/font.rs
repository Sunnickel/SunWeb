use std::str::FromStr;

/// Represents different font subtypes.
///
/// This enum covers common web and desktop font formats such as
/// WOFF, WOFF2, OTF, and TTF. Any other font subtype not explicitly
/// listed is captured by the `Other` variant.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// let f = FontSubType::from_str("woff2").unwrap();
/// assert_eq!(f, FontSubType::Woff2);
///
/// let unknown = FontSubType::from_str("bitmap").unwrap();
/// assert_eq!(unknown, FontSubType::Other("bitmap".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontSubType {
    /// WOFF (Web Open Font Format)
    Woff,
    /// WOFF2 (Web Open Font Format 2)
    Woff2,
    /// OTF (OpenType Font)
    Otf,
    /// TTF (TrueType Font)
    Ttf,
    /// Any other font subtype not listed above
    Other(String),
}

impl FromStr for FontSubType {
    type Err = ();

    /// Parses a string into a `FontSubType`.
    ///
    /// Recognizes `"woff"`, `"woff2"`, `"otf"`, and `"ttf"`.
    /// Any other string will be wrapped in `FontSubType::Other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr;
    /// use your_crate::FontSubType;
    ///
    /// let font = FontSubType::from_str("ttf").unwrap();
    /// assert_eq!(font, FontSubType::Ttf);
    /// ```
    fn from_str(sub: &str) -> Result<Self, Self::Err> {
        Ok(match sub {
            "woff" => FontSubType::Woff,
            "woff2" => FontSubType::Woff2,
            "otf" => FontSubType::Otf,
            "ttf" => FontSubType::Ttf,
            other => FontSubType::Other(other.into()),
        })
    }
}
