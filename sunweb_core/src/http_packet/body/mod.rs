use std::ops::Deref;
use std::collections::HashMap;
use crate::http_packet::requests::url_decode;

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

    /// Body as a UTF-8 string
    /// ```rust
    /// let text = req.body()?.as_string()?;
    /// ```
    pub fn as_string(&self) -> Option<String> {
        String::from_utf8(self.0.clone()).ok()
    }

    /// Deserialize body as JSON directly into a typed struct
    /// ```rust
    /// #[derive(Deserialize)]
    /// struct CreateUser { name: String, age: u32 }
    ///
    /// let user = req.body()?.as_json::<CreateUser>()?;
    /// println!("{}", user.name);
    /// ```
    pub fn as_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        let text = self.as_string().ok_or("Body is not valid UTF-8")?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    /// Parse as url-encoded form fields
    /// ```rust
    /// let fields = req.body()?.as_form()?;
    /// let name = fields.get("name")?;
    /// ```
    pub fn as_form(&self) -> Option<HashMap<String, String>> {
        let text = self.as_string()?;
        let mut map = HashMap::new();

        for pair in text.split('&') {
            if let Some(eq) = pair.find('=') {
                let key = url_decode(&pair[..eq]);
                let val = url_decode(&pair[eq + 1..]);
                map.insert(key, val);
            } else {
                map.insert(url_decode(pair), String::new());
            }
        }

        Some(map)
    }

    /// Raw bytes — same as dereffing but more explicit
    /// ```rust
    /// let bytes: &[u8] = req.body()?.as_bytes();
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Try to parse the body as any type that implements FromStr
    /// ```rust
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