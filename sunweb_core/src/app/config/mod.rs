use rustls::ServerConfig as RustlsConfig;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use std::sync::Arc;

/// Configuration for the web server.
///
/// This struct holds all the necessary information to configure a web server,
/// including network settings, TLS configuration, and domain information.
///
/// # Examples
///
/// ```rust
/// use sunweb::server::server_config::ServerConfig;
///
/// let config = ServerConfig::new([127, 0, 0, 1], 8080)
///     .set_base_domain("example.com".to_string());
/// ```
pub struct ServerConfig {
    /// The IP address the server will bind to.
    pub(crate) host: [u8; 4],
    /// The port number the server will listen on.
    pub(crate) port: u16,
    /// Indicates whether HTTPS is enabled for the server.
    pub(crate) using_https: bool,
    /// Optional TLS configuration for secure connections.
    pub(crate) tls_config: Option<Arc<RustlsConfig>>,
    /// The base domain used for the server. Defaults to localhost.
    pub(crate) base_domain: String,
}

impl ServerConfig {
    /// Creates a new `ServerConfig` with the specified host and port.
    ///
    /// By default, HTTPS is disabled, no TLS configuration is set,
    /// and the base domain is initialized to `"localhost"`.
    ///
    /// # Arguments
    ///
    /// * `host` - An array of 4 u8 values representing the IPv4 address.
    /// * `port` - The port number to listen on.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sunweb::server::server_config::ServerConfig;
    ///
    /// let config = ServerConfig::new([127, 0, 0, 1], 8080);
    /// ```
    pub fn new(host: [u8; 4], port: u16) -> ServerConfig {
        Self {
            host,
            port,
            using_https: false,
            tls_config: None,
            base_domain: String::from("localhost"),
        }
    }

    /// Adds TLS certificate configuration to the server.
    ///
    /// This method configures the server to use HTTPS with the provided private key and certificate files.
    /// The certificate file must be PEM-encoded, and the private key file must be PEM-encoded as well.
    ///
    /// # Arguments
    ///
    /// * `private_key_pem` - Path to the PEM file containing the private key.
    /// * `cert_pem` - Path to the PEM file containing the certificate(s).
    ///
    /// # Returns
    ///
    /// The updated `ServerConfig` with TLS enabled.
    ///
    /// # Panics
    ///
    /// Panics if the certificate or private key files cannot be read,
    /// or if the certificates are malformed or empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sunweb::server::server_config::ServerConfig;
    ///
    /// let config = ServerConfig::new([127, 0, 0, 1], 8080)
    ///     .add_cert("private_key.pem".to_string(), "cert.pem".to_string())
    ///     .expect("Failed to add certificate");
    /// ```
    pub fn add_cert(mut self, private_key_pem: String, cert_pem: String, http2: bool) -> Self {
        let certs: Result<Vec<_>, _> = CertificateDer::pem_file_iter(cert_pem)
            .unwrap()
            .collect::<Result<Vec<_>, _>>();
        let certs = certs.map_err(|e| format!("Failed to parse certificates: {}", e));
        let key: PrivateKeyDer = PrivateKeyDer::from_pem_file(private_key_pem).unwrap();

        if certs.clone().unwrap().is_empty() {
            panic!("Failed to parse certificates");
        }

        let mut tls_config = RustlsConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs.unwrap(), key)
            .map_err(|e| format!("Failed to create TLS config: {}", e))
            .unwrap();

        if http2 {
            tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        } else {
            tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        }

        self.tls_config = Some(Arc::new(tls_config));
        self.using_https = true;

        self
    }
    /// Sets the base domain for the server.
    ///
    /// This domain is used as a default for operations like generating URLs,
    /// handling cookies, and subdomain routing.
    ///
    /// # Arguments
    ///
    /// * `base_domain` - The base domain string to set.
    ///
    /// # Returns
    ///
    /// The updated `ServerConfig` with the new base domain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sunweb::server::server_config::ServerConfig;
    ///
    /// let config = ServerConfig::new([127, 0, 0, 1], 8080)
    ///     .set_base_domain("example.com".to_string());
    /// ```
    pub fn set_base_domain(mut self, base_domain: String) -> Self {
        self.base_domain = base_domain;
        self
    }
}
