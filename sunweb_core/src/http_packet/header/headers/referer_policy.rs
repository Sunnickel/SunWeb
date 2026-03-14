/// Referrer-Policy options
#[derive(Clone, Debug)]
pub enum ReferrerPolicy {
    /// No referrer information
    NoReferrer,
    /// No referrer when downgrading from HTTPS to HTTP
    NoReferrerWhenDowngrade,
    /// Only send origin
    Origin,
    /// Origin only when cross-origin
    OriginWhenCrossOrigin,
    /// Same origin only
    SameOrigin,
    /// Strict origin
    StrictOrigin,
    /// Strict origin when cross-origin
    StrictOriginWhenCrossOrigin,
    /// Unsafe URL (send full URL)
    UnsafeUrl,
}

impl ReferrerPolicy {
    pub fn as_str(&self) -> &str {
        match self {
            ReferrerPolicy::NoReferrer => "no-referrer",
            ReferrerPolicy::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            ReferrerPolicy::Origin => "origin",
            ReferrerPolicy::OriginWhenCrossOrigin => "origin-when-cross-origin",
            ReferrerPolicy::SameOrigin => "same-origin",
            ReferrerPolicy::StrictOrigin => "strict-origin",
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            ReferrerPolicy::UnsafeUrl => "unsafe-url",
        }
    }
}
