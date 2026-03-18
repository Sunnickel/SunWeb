use crate::http_packet::requests::url_decode;
use std::collections::HashMap;
use std::ops::Deref;

/// The raw body of an HTTP request.
///
/// Obtain one via [`HTTPRequest::body`] — it returns `None` if no body was sent.
/// `Body` derefs to `Vec<u8>` for direct byte access.
pub struct Body(Vec<u8>);

impl Deref for Body {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Body {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Body(bytes)
    }

    /// Decodes the body as a UTF-8 string, returning `None` if the bytes are
    /// not valid UTF-8.
    pub fn as_string(&self) -> Option<String> {
        String::from_utf8(self.0.clone()).ok()
    }

    /// Deserializes the body as JSON into `T`.
    ///
    /// # Errors
    /// Returns `Err` if the body is not valid UTF-8 or fails to deserialize.
    ///
    /// ```rust,ignore
    /// #[derive(Deserialize)]
    /// struct CreateUser { name: String, age: u32 }
    ///
    /// let user = req.body()?.as_json::<CreateUser>()?;
    /// ```
    pub fn as_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        let text = self.as_string().ok_or("Body is not valid UTF-8")?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    /// Parses the body as `application/x-www-form-urlencoded` key-value pairs.
    ///
    /// Returns `None` if the body is not valid UTF-8.
    ///
    /// ```rust,ignore
    /// let fields = req.body()?.as_form()?;
    /// let name = fields.get("name")?;
    /// ```
    pub fn as_form(&self) -> Option<HashMap<String, String>> {
        let text = self.as_string()?;
        let mut map = HashMap::new();

        for pair in text.split('&') {
            if let Some(eq) = pair.find('=') {
                map.insert(url_decode(&pair[..eq]), url_decode(&pair[eq + 1..]));
            } else {
                map.insert(url_decode(pair), String::new());
            }
        }

        Some(map)
    }

    /// Returns the raw bytes of the body.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Parses the body into any type that implements [`FromStr`](std::str::FromStr).
    ///
    /// # Errors
    /// Returns `Err` if the body is not valid UTF-8 or parsing fails.
    ///
    /// ```rust,ignore
    /// let id: u64 = req.body()?.parse::<u64>()?;
    /// ```
    pub fn parse<T: std::str::FromStr>(&self) -> Result<T, String>
    where
        T::Err: std::fmt::Display,
    {
        self.as_string()
            .ok_or("Body is not valid UTF-8".to_string())?
            .parse::<T>()
            .map_err(|e| e.to_string())
    }
}
