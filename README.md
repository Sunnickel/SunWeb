# Project: sunweb
> Sunnickel | 04.10.2025
---
A lightweight, fast, and flexible HTTP/HTTPS web server written in Rust with domain routing, middleware support, and TLS
capabilities.

## Features

- **HTTP/HTTPS Support**: Built-in TLS/SSL support using rustls
- **Domain-Based Routing**: Multi-domain and subdomain routing capabilities
- **Middleware System**: Flexible middleware for request/response processing
- **Static File Serving**: Efficient static file serving with automatic MIME type detection
- **Custom Route Handlers**: Define custom logic for specific routes
- **Custom Error Managment**: Define sites that get shown when specific error get thrown
- **Cookie Management**: Full cookie support with security attributes
- **Colored Logging**: Built-in colored console logging for better visibility
- **Thread-Per-Connection**: Handles each client connection in a separate thread

## Installation

Add this to your `Cargo.toml`:

```toml 
[dependencies] sunweb = "0.2.1"
``` 

Or install from GitHub:

```toml 
[dependencies] sunweb = { git = "https://github.com/Sunnickel/SunWeb" }
``` 

## Quick Start

```rust 
use sunweb::webserver::{WebServer, ServerConfig};
use sunweb::WEB_LOGGER;

fn main() {
    // Initialize logger 
    log::set_logger(&WEB_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    // Create server configuration
    let config = ServerConfig::new([127, 0, 0, 1], 8080)
        .set_base_domain("localhost".to_string());

    // Create and configure server
    let mut server = WebServer::new(config);

    // Add a simple route
    server.add_route_file("/", "./static/index.html", None).unwrap();

    // Start the server
    server.start();
}
```

## Usage Examples

### Basic HTTP Server

```rust
use sunweb::webserver::{WebServer, ServerConfig};
let config = ServerConfig::new([127, 0, 0, 1], 8080);
let server = WebServer::new(config); server.start();
```

### HTTPS Server with TLS

```rust 
let config = ServerConfig::new([127, 0, 0, 1], 443).add_cert("private_key.pem".to_string(), "cert.pem".to_string()).expect("Failed to load certificates");
let server = WebServer::new(config); server.start();
``` 

### Custom Route Handlers

```rust
use sunweb::webserver::responses::{Response, ResponseCodes};
use std::sync::Arc;

server.add_custom_route("/api/hello", | req, domain| {
let content = Arc::new(String::from(r#"{"message": "Hello, World!"}"#));
let mut response = Response::new(content, Some(ResponseCodes::Ok), None);
response.headers.add_header("Content-Type", "application/json");
response
}, None).unwrap();
```

Static File Serving

```rust
// Serve all files in the ./static folder under /static route
server.add_static_route("/static", "./static", None).unwrap();
```

Working with Cookies

```rust
use sunweb::webserver::cookie::{Cookie, SameSite};
use sunweb::webserver::Domain;

server.add_custom_route("/set-cookie", | req, domain| {
let content = Arc::new(String::from("Cookie set!"));
let mut response = Response::new(content, None, None);

let cookie = Cookie::new("session_id", "abc123", domain)
.expires(Some(3600))  // 1 hour
.secure()
.http_only()
.same_site(SameSite::Strict);

response.add_cookie(cookie);
response

}, None).unwrap();
```

Subdomain Routing

```rust
use sunweb::webserver::Domain;

let api_domain = Domain::new("api");
server.add_subdomain_router( & api_domain);

// Add routes specifically for api.yourdomain.com
server.add_custom_route("/users", | req, domain| {
// API logic here
Response::new(
Arc::new(String::from(r#"{"users": []}"#)),
Some(ResponseCodes::Ok),
None
)
}, Some( & api_domain)).unwrap();
```

## API Overview

### ServerConfig

Configure your web server:

- ```new(host: [u8; 4], port: u16)``` - Create new configuration
- ```add_cert(private_key: String, cert: String)``` - Enable HTTPS
- ```set_base_domain(domain: String)``` - Set the base domain

### WebServer

Main server instance:

- ```new(config: ServerConfig)``` - Create new server
- ```start()``` - Start listening for connections
- ```add_route_file(route: &str, file: &str, domain: Option<&Domain>)``` - Add file route
- ```add_static_route(route: &str, folder: &str, domain: Option<&Domain>)``` - Add static folder
- ```add_custom_route(route: &str, handler: Fn, domain: Option<&Domain>)``` - Add custom handler
- ```add_subdomain_router(domain: &Domain)``` - Enable subdomain routing
- ```add_error_route(status_code: ResponseCodes, file: &str, domain: Option<&Domain>)``` - Add custom error-pages

### Response

HTTP response handling:

- ```new(content: Arc<String>, code: Option<ResponseCodes>, protocol: Option<String>)``` - Create response
- ```add_cookie(cookie: Cookie)``` - Add cookie to response
- ```headers.add_header(key: &str, value: &str)``` - Add custom header

### Request

HTTP request information:

- ```protocol: String``` - HTTP protocol version
- ```method: String``` - HTTP method (GET, POST, etc.)
- ```route: String``` - Requested route
- ```values: HashMap<String, String>``` - Request headers
- ```remote_addr: String``` - Client IP address
- ```get_cookies()``` - Get all cookies
- ```get_cookie(key: &str)``` - Get specific cookie

## Project Structure

```
RustWebservice/
├── src/
│ ├── webserver/
│ │ ├── client_handling/               # Client connection handling
│ │ ├── cookie/                        # Cookie management
│ │ ├── files/                         # Static file serving
│ │ ├── logger/                        # Colored logging
│ │ ├── middleware/                    # Middleware system
│ │ ├── requests/                      # Request parsing
│ │ ├── responses/                     # Response building
│ │ └── server_config/                 # Server configuration
│ └── lib.rs
├── Cargo.toml
└── README.md
```

## Dependencies

* **chrono** - Date and time handling
* **log** - Logging facade
* **rustls** - TLS/SSL implementation
* **rustls-pki-types** - PKI types for rustls

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Author

Sunnickel

## Links

- [Repository](https://github.com/Sunnickel/SunWeb)
- [Documentation](https://docs.rs/sunweb/latest/sunweb)

## AI Assistance

This project's documentation, including this `README.md` file, has been created with the assistance of AI.

Specifically, AI was used for:

*   Documentation generation
*   Content creation for this `README.md`

All content has been reviewed and edited to ensure accuracy and clarity. While AI tools were utilized to accelerate the writing process, all information has been manually checked for correctness.
