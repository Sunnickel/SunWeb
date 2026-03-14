/// Content-Encoding types
#[derive(Clone, Debug)]
pub enum ContentEncoding {
    /// Gzip compression
    Gzip,
    /// Deflate compression
    Deflate,
    /// Brotli compression
    Brotli,
    /// Identity (no encoding)
    Identity,
    /// Multiple encodings applied in order
    Multiple(Vec<ContentEncoding>),
}

impl ContentEncoding {
    pub fn as_str(&self) -> String {
        match self {
            ContentEncoding::Gzip => "gzip".to_string(),
            ContentEncoding::Deflate => "deflate".to_string(),
            ContentEncoding::Brotli => "br".to_string(),
            ContentEncoding::Identity => "identity".to_string(),
            ContentEncoding::Multiple(encodings) => {
                encodings
                    .iter()
                    .map(|e| e.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
    }
}