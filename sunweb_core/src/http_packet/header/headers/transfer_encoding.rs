/// Transfer-Encoding types
#[derive(Clone, Debug)]
pub enum TransferEncoding {
    /// Chunked transfer encoding
    Chunked,
    /// Compress
    Compress,
    /// Deflate
    Deflate,
    /// Gzip
    Gzip,
    /// Multiple encodings
    Multiple(Vec<TransferEncoding>),
}

impl TransferEncoding {
    pub fn as_str(&self) -> String {
        match self {
            TransferEncoding::Chunked => "chunked".to_string(),
            TransferEncoding::Compress => "compress".to_string(),
            TransferEncoding::Deflate => "deflate".to_string(),
            TransferEncoding::Gzip => "gzip".to_string(),
            TransferEncoding::Multiple(encodings) => {
                encodings
                    .iter()
                    .map(|e| e.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
    }
}
