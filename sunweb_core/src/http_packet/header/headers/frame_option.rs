/// Options for X-Frame-Options header
#[derive(Clone, Debug)]
pub enum FrameOption {
    /// DENY - Cannot be displayed in a frame
    Deny,
    /// SAMEORIGIN - Can only be displayed in a frame on the same origin
    SameOrigin,
    /// ALLOW-FROM uri - Can only be displayed in a frame on the specified origin
    AllowFrom(String),
}

impl FrameOption {
    pub fn as_str(&self) -> &str {
        match self {
            FrameOption::Deny => "DENY",
            FrameOption::SameOrigin => "SAMEORIGIN",
            FrameOption::AllowFrom(uri) => uri,
        }
    }
}
