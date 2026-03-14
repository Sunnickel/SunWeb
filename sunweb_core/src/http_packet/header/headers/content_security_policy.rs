/// Content-Security-Policy directives
#[derive(Clone, Debug)]
pub enum CspDirective {
    /// default-src
    DefaultSrc(Vec<String>),
    /// script-src
    ScriptSrc(Vec<String>),
    /// style-src
    StyleSrc(Vec<String>),
    /// img-src
    ImgSrc(Vec<String>),
    /// font-src
    FontSrc(Vec<String>),
    /// connect-src
    ConnectSrc(Vec<String>),
    /// frame-src
    FrameSrc(Vec<String>),
    /// object-src
    ObjectSrc(Vec<String>),
    /// media-src
    MediaSrc(Vec<String>),
    /// Custom directive
    Custom(String, Vec<String>),
}

impl CspDirective {
    pub fn as_str(&self) -> String {
        match self {
            CspDirective::DefaultSrc(sources) => format!("default-src {}", sources.join(" ")),
            CspDirective::ScriptSrc(sources) => format!("script-src {}", sources.join(" ")),
            CspDirective::StyleSrc(sources) => format!("style-src {}", sources.join(" ")),
            CspDirective::ImgSrc(sources) => format!("img-src {}", sources.join(" ")),
            CspDirective::FontSrc(sources) => format!("font-src {}", sources.join(" ")),
            CspDirective::ConnectSrc(sources) => format!("connect-src {}", sources.join(" ")),
            CspDirective::FrameSrc(sources) => format!("frame-src {}", sources.join(" ")),
            CspDirective::ObjectSrc(sources) => format!("object-src {}", sources.join(" ")),
            CspDirective::MediaSrc(sources) => format!("media-src {}", sources.join(" ")),
            CspDirective::Custom(name, sources) => format!("{} {}", name, sources.join(" ")),
        }
    }
}

/// Builds a CSP policy from multiple directives
pub struct CspBuilder {
    directives: Vec<CspDirective>,
}

impl CspBuilder {
    pub fn new() -> Self {
        Self {
            directives: Vec::new(),
        }
    }

    pub fn directive(mut self, directive: CspDirective) -> Self {
        self.directives.push(directive);
        self
    }

    pub fn build(&self) -> String {
        self.directives
            .iter()
            .map(|directive: &CspDirective| directive.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }
}
