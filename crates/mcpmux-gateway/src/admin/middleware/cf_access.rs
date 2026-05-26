//! Cloudflare Access JWT validation for the admin HTTP server.
//!
//! When `trust_cf_access` is enabled, requests must include a valid
//! `CF-Access-Jwt-Assertion` header signed by Cloudflare team certs.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::sync::Arc;
use thiserror::Error;
use tracing::debug;

use super::super::config::CF_ACCESS_JWT_HEADER;
use super::super::router::AdminState;

/// PEM-encoded RSA public key used only by hidden test helpers.
const TEST_RSA_PUBLIC_PEM: &str =
    include_str!("../../../../../tests/fixtures/cf_access_test_pubkey.pem");

/// PEM-encoded RSA private key used only by hidden test helpers.
const TEST_RSA_PRIVATE_PEM: &str =
    include_str!("../../../../../tests/fixtures/cf_access_test_private.pem");

/// Errors from CF Access JWT validation.
#[derive(Debug, Error)]
pub enum CfAccessError {
    /// JWT header or signature could not be parsed or verified.
    #[error("invalid JWT: {0}")]
    InvalidJwt(String),
    /// No matching decoding key for the token `kid`.
    #[error("unknown key id: {0}")]
    UnknownKeyId(String),
    /// Cert fetch or configuration error.
    #[error("{0}")]
    Config(String),
}

/// Validated Cloudflare Access JWT claims (subset used for checks).
#[derive(Debug, Deserialize)]
pub struct CfAccessClaims {
    /// Subject (user email or service identity).
    pub sub: String,
    /// Issuer (`https://<team>.cloudflareaccess.com`).
    pub iss: String,
    /// Audience (application AUD tag).
    pub aud: serde_json::Value,
    /// Expiration (unix seconds).
    pub exp: i64,
}

impl std::fmt::Debug for CfAccessValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CfAccessValidator")
            .field("keys", &self.keys.len())
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .finish()
    }
}

/// Validates `CF-Access-Jwt-Assertion` tokens against Cloudflare team certs.
pub struct CfAccessValidator {
    keys: Vec<DecodingKey>,
    issuer: Option<String>,
    audience: Option<String>,
}

impl CfAccessValidator {
    /// Build a validator from PEM-encoded RSA public certificates.
    pub fn from_pem_certs(
        certs: Vec<String>,
        issuer: Option<String>,
        audience: Option<String>,
    ) -> Result<Self, CfAccessError> {
        let mut keys = Vec::new();
        for cert in certs {
            let key = DecodingKey::from_rsa_pem(cert.as_bytes())
                .map_err(|e| CfAccessError::Config(format!("invalid PEM cert: {e}")))?;
            keys.push(key);
        }
        if keys.is_empty() {
            return Err(CfAccessError::Config("no certificates provided".into()));
        }
        Ok(Self {
            keys,
            issuer,
            audience,
        })
    }

    /// Fetch team certs from Cloudflare and build a validator.
    pub async fn from_team_domain(
        team_domain: &str,
        audience: Option<String>,
    ) -> Result<Self, CfAccessError> {
        let url = format!("https://{team_domain}.cloudflareaccess.com/cdn-cgi/access/certs");
        let issuer = format!("https://{team_domain}.cloudflareaccess.com");
        let response = reqwest::get(&url)
            .await
            .map_err(|e| CfAccessError::Config(format!("cert fetch failed: {e}")))?;
        if !response.status().is_success() {
            return Err(CfAccessError::Config(format!(
                "cert fetch returned {}",
                response.status()
            )));
        }
        let body: CertsResponse = response
            .json()
            .await
            .map_err(|e| CfAccessError::Config(format!("cert JSON parse failed: {e}")))?;
        Self::from_pem_certs(body.public_certs, Some(issuer), audience)
    }

    /// Validate a JWT string and return decoded claims.
    pub fn validate(&self, token: &str) -> Result<CfAccessClaims, CfAccessError> {
        let header = decode_header(token).map_err(|e| CfAccessError::InvalidJwt(e.to_string()))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        if let Some(ref iss) = self.issuer {
            validation.set_issuer(&[iss.as_str()]);
        }
        if let Some(ref aud) = self.audience {
            validation.set_audience(&[aud.as_str()]);
        }

        for key in &self.keys {
            if let Ok(token_data) = decode::<CfAccessClaims>(token, key, &validation) {
                debug!(sub = %token_data.claims.sub, "CF Access JWT validated");
                return Ok(token_data.claims);
            }
        }

        let kid = header.kid.unwrap_or_else(|| "unknown".into());
        Err(CfAccessError::UnknownKeyId(kid))
    }
}

#[derive(Debug, Deserialize)]
struct CertsResponse {
    #[serde(default)]
    public_certs: Vec<String>,
}

/// Axum middleware: require valid CF Access JWT when enabled in config.
pub async fn cf_access_middleware(
    State(state): State<AdminState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    if !state.config.trust_cf_access {
        return next.run(request).await;
    }

    let Some(validator) = state.cf_validator.as_ref() else {
        return cf_access_unauthorized("CF Access validation not configured");
    };

    let token = headers
        .get(CF_ACCESS_JWT_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match token {
        Some(jwt) => match validator.validate(jwt) {
            Ok(_) => next.run(request).await,
            Err(e) => {
                debug!(error = %e, "CF Access JWT rejected");
                cf_access_unauthorized("invalid CF Access token")
            }
        },
        None => cf_access_unauthorized("missing CF-Access-Jwt-Assertion"),
    }
}

fn cf_access_unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

/// Test-only validator backed by the repo fixture key pair.
#[doc(hidden)]
pub fn test_validator() -> Arc<CfAccessValidator> {
    Arc::new(
        CfAccessValidator::from_pem_certs(
            vec![TEST_RSA_PUBLIC_PEM.to_string()],
            Some("https://test.cloudflareaccess.com".into()),
            Some("test-audience".into()),
        )
        .expect("test validator"),
    )
}

/// Test-only signed JWT accepted by [`test_validator`].
#[doc(hidden)]
pub fn test_valid_jwt() -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};

    let claims = serde_json::json!({
        "sub": "test@example.com",
        "iss": "https://test.cloudflareaccess.com",
        "aud": "test-audience",
        "exp": chrono::Utc::now().timestamp() + 3600,
    });
    let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_PEM.as_bytes()).expect("test private key");
    encode(&Header::new(Algorithm::RS256), &claims, &key).expect("sign test jwt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_test_fixture_jwt() {
        let validator = test_validator();
        let jwt = test_valid_jwt();
        assert!(validator.validate(&jwt).is_ok());
    }

    #[test]
    fn validate_rejects_missing_signature() {
        let validator = test_validator();
        let err = validator.validate("not.a.jwt").unwrap_err();
        assert!(matches!(err, CfAccessError::InvalidJwt(_)));
    }

    #[test]
    fn validate_rejects_wrong_audience() {
        let validator = CfAccessValidator::from_pem_certs(
            vec![TEST_RSA_PUBLIC_PEM.to_string()],
            Some("https://test.cloudflareaccess.com".into()),
            Some("other-audience".into()),
        )
        .unwrap();
        let jwt = test_valid_jwt();
        let err = validator.validate(&jwt).unwrap_err();
        assert!(matches!(err, CfAccessError::UnknownKeyId(_)));
    }

    #[test]
    fn from_pem_certs_rejects_empty_list() {
        let err = CfAccessValidator::from_pem_certs(vec![], None, None).unwrap_err();
        assert!(matches!(err, CfAccessError::Config(_)));
    }

    #[test]
    fn from_pem_certs_rejects_invalid_pem() {
        let err =
            CfAccessValidator::from_pem_certs(vec!["not-a-cert".into()], None, None).unwrap_err();
        assert!(matches!(err, CfAccessError::Config(_)));
    }
}
