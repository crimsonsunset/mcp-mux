//! Admin server configuration.

/// Default admin listen port (loopback + CF tunnel).
pub const DEFAULT_ADMIN_PORT: u16 = 45819;

/// Cloudflare Access JWT header forwarded by the tunnel edge.
pub const CF_ACCESS_JWT_HEADER: &str = "CF-Access-Jwt-Assertion";

/// Admin HTTP server configuration.
#[derive(Debug, Clone)]
pub struct AdminConfig {
    /// Host to bind to (default loopback).
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Require and validate `CF-Access-Jwt-Assertion` when true.
    pub trust_cf_access: bool,
    /// Cloudflare team domain for JWT cert validation (Phase 2).
    pub cf_team_domain: Option<String>,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: DEFAULT_ADMIN_PORT,
            trust_cf_access: false,
            cf_team_domain: None,
        }
    }
}

impl AdminConfig {
    /// Socket address string for binding.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
