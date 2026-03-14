use chrono::{Duration, Utc};
/// Represents the SameSite attribute for cookies.
///
/// This enum defines the SameSite policy that governs how cookies are sent with cross-site requests.
#[derive(Clone, Copy, Debug)]
pub enum SameSite {
    /// Allows cookies to be sent with all requests, including cross-site requests.
    None,
    /// Allows cookies to be sent with same-site requests and top-level navigations.
    Lax,
    /// Only allows cookies to be sent with same-site requests.
    Strict,
}

/// A cookie representation for HTTP responses.
///
/// This struct provides a way to construct and serialize HTTP cookies according to RFC 6265.
/// It supports various cookie attributes like path, domain, secure flag, HttpOnly flag, etc.
#[derive(Clone, Debug)]
pub struct Cookie {
    /// The name of the cookie.
    pub(crate) key: String,
    /// The value of the cookie.
    value: String,
    /// The maximum age of the cookie in seconds.
    max_age: Option<u64>,
    /// The path for which the cookie is valid.
    path: String,
    /// The domain for which the cookie is valid.
    domain: String,
    /// The SameSite policy for the cookie.
    same_site: SameSite,
    /// Whether the cookie should only be sent over secure (HTTPS) connections.
    secure: bool,
    /// Whether the cookie should be accessible only through the HTTP protocol.
    is_http_only: bool,
}

impl Cookie {
    /// Creates a new `Cookie` instance.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the cookie.
    /// * `value` - The value of the cookie.
    /// * `domain` - The domain for which the cookie is valid.
    ///
    /// # Returns
    ///
    /// A new `Cookie` instance with default values for most attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// use webserver::Domain;
    /// use webserver::cookie::Cookie;
    ///
    /// let domain = Domain::new("example.com");
    /// let cookie = Cookie::new("session_id", "abc123", &domain);
    /// ```
    pub fn new(key: &str, value: &str, domain: &String) -> Cookie {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            max_age: None,
            path: "/".to_string(),
            domain: domain.clone(),
            same_site: SameSite::Lax, // sensible default
            secure: false,
            is_http_only: false,
        }
    }

    /// Converts the cookie to its string representation.
    ///
    /// This method formats all cookie attributes into a single string that can be used in an HTTP `Set-Cookie` header.
    ///
    /// # Returns
    ///
    /// A formatted string representing the cookie with all its attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// use webserver::Domain;
    /// use webserver::cookie::{Cookie, SameSite};
    ///
    /// let domain = Domain::new("example.com");
    /// let cookie = Cookie::new("session_id", "abc123", &domain)
    ///     .expires(Some(3600))
    ///     .secure()
    ///     .http_only();
    ///
    /// assert_eq!(cookie.as_string(), "session_id=abc123; Max-Age=3600; Expires=...; Path=/; Domain=example.com; SameSite=Lax; Secure; HttpOnly");
    /// ```
    pub(crate) fn as_string(&self) -> String {
        let mut base = format!("{}={}; ", self.key, self.value);
        if let Some(seconds) = self.max_age {
            base.push_str(&format!("Max-Age={}; ", seconds));
            let expires = Utc::now() + Duration::seconds(seconds as i64);
            base.push_str(&format!(
                "Expires={}; ",
                expires.format("%a, %d %b %Y %H:%M:%S GMT")
            ));
        }
        base.push_str(&format!("Path={}; ", self.path));
        base.push_str(&format!("Domain={}; ", &self.domain));
        let same_site_str = match self.same_site {
            SameSite::None => "None",
            SameSite::Lax => "Lax",
            SameSite::Strict => "Strict",
        };
        base.push_str(&format!("SameSite={}; ", same_site_str));
        if self.secure {
            base.push_str("Secure; ");
        }
        if self.is_http_only {
            base.push_str("HttpOnly; ");
        }
        base.trim_end().to_string()
    }

    /// Sets the maximum age of the cookie.
    ///
    /// # Arguments
    ///
    /// * `max_age` - The maximum age in seconds, or None to unset.
    ///
    /// # Returns
    ///
    /// The modified `Cookie` instance for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use webserver::Domain;
    /// use webserver::cookie::Cookie;
    ///
    /// let domain = Domain::new("example.com");
    /// let cookie = Cookie::new("session_id", "abc123", &domain).expires(Some(3600));
    /// ```
    pub fn expires(mut self, max_age: Option<u64>) -> Self {
        self.max_age = max_age;
        self
    }

    /// Marks the cookie as secure.
    ///
    /// This ensures that the cookie is only sent over HTTPS connections.
    ///
    /// # Returns
    ///
    /// The modified `Cookie` instance for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use webserver::Domain;
    /// use webserver::cookie::Cookie;
    ///
    /// let domain = Domain::new("example.com");
    /// let cookie = Cookie::new("session_id", "abc123", &domain).secure();
    /// ```
    pub fn secure(mut self) -> Self {
        self.secure = true;
        self
    }

    /// Marks the cookie as HTTP only.
    ///
    /// This prevents client-side scripts from accessing the cookie, mitigating XSS attacks.
    ///
    /// # Returns
    ///
    /// The modified `Cookie` instance for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use webserver::Domain;
    /// use webserver::cookie::Cookie;
    ///
    /// let domain = Domain::new("example.com");
    /// let cookie = Cookie::new("session_id", "abc123", &domain).http_only();
    /// ```
    pub fn http_only(mut self) -> Self {
        self.is_http_only = true;
        self
    }

    /// Sets the path for which the cookie is valid.
    ///
    /// # Arguments
    ///
    /// * `path` - The path string for the cookie.
    ///
    /// # Returns
    ///
    /// The modified `Cookie` instance for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use webserver::Domain;
    /// use webserver::cookie::Cookie;
    ///
    /// let domain = Domain::new("example.com");
    /// let cookie = Cookie::new("session_id", "abc123", &domain).path("/admin");
    /// ```
    pub fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }

    /// Sets the SameSite policy for the cookie.
    ///
    /// # Arguments
    ///
    /// * `same_site` - The SameSite policy to apply.
    ///
    /// # Returns
    ///
    /// The modified `Cookie` instance for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use webserver::Domain;
    /// use webserver::cookie::{Cookie, SameSite};
    ///
    /// let domain = Domain::new("example.com");
    /// let cookie = Cookie::new("session_id", "abc123", &domain).same_site(SameSite::Strict);
    /// ```
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }
}

impl FromIterator<bool> for Cookie {
    fn from_iter<I: IntoIterator<Item = bool>>(iter: I) -> Self {
        iter.into_iter().collect()
    }
}
