/// Cache-Control directives
#[derive(Clone, Debug)]
pub enum CacheControl {
    /// No caching at all
    NoCache,
    /// No store - don't store in cache
    NoStore,
    /// Must revalidate before using cached version
    MustRevalidate,
    /// Public - can be cached by any cache
    Public,
    /// Private - can only be cached by browser
    Private,
    /// Max age in seconds
    MaxAge(u64),
    /// S-maxage for shared caches
    SMaxAge(u64),
    /// No transform
    NoTransform,
    /// Combination of multiple directives
    Multiple(Vec<CacheControl>),
}

impl CacheControl {
    pub fn as_str(&self) -> String {
        match self {
            CacheControl::NoCache => "no-cache".to_string(),
            CacheControl::NoStore => "no-store".to_string(),
            CacheControl::MustRevalidate => "must-revalidate".to_string(),
            CacheControl::Public => "public".to_string(),
            CacheControl::Private => "private".to_string(),
            CacheControl::MaxAge(seconds) => format!("max-age={}", seconds),
            CacheControl::SMaxAge(seconds) => format!("s-maxage={}", seconds),
            CacheControl::NoTransform => "no-transform".to_string(),
            CacheControl::Multiple(directives) => {
                directives
                    .iter()
                    .map(|d| d.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
    }
}
