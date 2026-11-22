//! `AuthClient` wrapper that keeps token failures classifiable.
//!
//! rmcp 1.5 folds every failed refresh into `AuthorizationRequired` and reports a 401 without a
//! `WWW-Authenticate` header as a plain `UnexpectedServerResponse`. Both lose the information
//! `HttpTransport::is_token_rejection` needs to tell a rejected token from a transient failure.

use std::collections::HashMap;
use std::sync::Arc;

use futures::stream::BoxStream;
use http::{HeaderName, HeaderValue};
use oauth2::TokenResponse;
use rmcp::model::ClientJsonRpcMessage;
use rmcp::transport::auth::{AuthClient, AuthError};
use rmcp::transport::streamable_http_client::{
    StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
};
use sse_stream::{Error as SseError, Sse};

/// OAuth error codes that mean the stored grant or client is permanently unusable.
const REJECTED_OAUTH_ERRORS: [&str; 3] = ["invalid_grant", "invalid_client", "unauthorized_client"];

/// Prefix of oauth2's `RequestTokenError::ServerResponse` display (the AS answered with an error).
const SERVER_RESPONSE_PREFIX: &str = "Server returned error response: ";

/// rmcp's `TokenRefreshFailed` reason when the stored token has no refresh token to use.
const NO_REFRESH_TOKEN_REASON: &str = "No refresh token available";

/// Whether a `TokenRefreshFailed` reason is a definitive rejection rather than a transient failure.
///
/// Network errors, unparsable responses and `server_error`/`temporarily_unavailable` answers are
/// transient. Only an AS error code naming a dead grant or client, or a missing refresh token,
/// counts.
pub(super) fn is_definitive_refresh_rejection(reason: &str) -> bool {
    if reason == NO_REFRESH_TOKEN_REASON {
        return true;
    }
    let Some(detail) = reason.strip_prefix(SERVER_RESPONSE_PREFIX) else {
        return false;
    };
    let code = detail.split([':', ' ']).next().unwrap_or_default().trim();
    REJECTED_OAUTH_ERRORS.contains(&code)
}

/// Status code of rmcp's `"HTTP {status}: {body}"` unexpected-response message, if it has one.
fn unexpected_response_status(message: &str) -> Option<u16> {
    // ponytail: rmcp 1.5 keeps the status only inside this message. Anchored to its own prefix
    // so body text can't match. Replace with a structured status once rmcp exposes one.
    let rest = message.strip_prefix("HTTP ")?;
    rest.get(..3)?.parse().ok()
}

/// Promote a header-less HTTP 401 to the same typed error a challenged 401 produces.
fn promote_unauthorized<E>(error: StreamableHttpError<E>) -> StreamableHttpError<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    match &error {
        StreamableHttpError::UnexpectedServerResponse(message)
            if unexpected_response_status(message) == Some(401) =>
        {
            StreamableHttpError::Auth(AuthError::AuthorizationRequired)
        }
        _ => error,
    }
}

/// `AuthClient` that surfaces the real refresh outcome and typed 401s.
#[derive(Clone)]
pub(super) struct RejectionAwareClient<C> {
    inner: AuthClient<C>,
}

impl<C> RejectionAwareClient<C> {
    pub(super) fn new(inner: AuthClient<C>) -> Self {
        Self { inner }
    }

    /// Access token, refreshing when needed. When rmcp reports `AuthorizationRequired`, the
    /// refresh is replayed once so its raw outcome (a new token, or the real failure reason)
    /// replaces rmcp's flattened error.
    async fn access_token<E>(&self) -> Result<String, StreamableHttpError<E>>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        match self.inner.get_access_token().await {
            Err(AuthError::AuthorizationRequired) => {}
            other => return other.map_err(Into::into),
        }
        let refreshed = self.inner.auth_manager.lock().await.refresh_token().await;
        Ok(refreshed?.access_token().secret().to_string())
    }
}

impl<C> StreamableHttpClient for RejectionAwareClient<C>
where
    C: StreamableHttpClient + Send + Sync,
{
    type Error = C::Error;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        let auth_token = match auth_token {
            Some(token) => token,
            None => self.access_token().await?,
        };
        self.inner
            .http_client
            .post_message(uri, message, session_id, Some(auth_token), custom_headers)
            .await
            .map_err(promote_unauthorized)
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        let auth_token = match auth_token {
            Some(token) => token,
            None => self.access_token().await?,
        };
        self.inner
            .http_client
            .delete_session(uri, session_id, Some(auth_token), custom_headers)
            .await
    }

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, StreamableHttpError<Self::Error>> {
        let auth_token = match auth_token {
            Some(token) => token,
            None => self.access_token().await?,
        };
        self.inner
            .http_client
            .get_stream(
                uri,
                session_id,
                last_event_id,
                Some(auth_token),
                custom_headers,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, routing::post, Router};
    use oauth2::basic::BasicTokenType;
    use oauth2::AccessToken;
    use rmcp::model::{ClientRequest, PingRequest};
    use rmcp::transport::auth::{
        AuthorizationManager, AuthorizationMetadata, InMemoryCredentialStore, OAuthTokenResponse,
        StoredCredentials,
    };

    #[test]
    fn refresh_rejection_only_for_dead_grants() {
        for reason in [
            "Server returned error response: invalid_grant: expired",
            "Server returned error response: invalid_client",
            "Server returned error response: invalid_grant (see https://example.com/help)",
            "Server returned error response: invalid_grant: expired (see https://example.com/help)",
            "No refresh token available",
        ] {
            assert!(is_definitive_refresh_rejection(reason), "{reason}");
        }
        for reason in [
            "Request failed",
            "Failed to parse server response",
            "Server returned error response: temporarily_unavailable",
            "Server returned error response: temporarily_unavailable (see https://example.com/help)",
            "Other error: invalid_grant",
        ] {
            assert!(!is_definitive_refresh_rejection(reason), "{reason}");
        }
    }

    /// Serve `app` on an ephemeral port and return its base URL.
    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    /// `AuthClient` holding an expired token whose refresh goes to `token_endpoint`.
    async fn expired_client(token_endpoint: &str) -> RejectionAwareClient<reqwest::Client> {
        let mut manager = AuthorizationManager::new("http://127.0.0.1:1/mcp")
            .await
            .unwrap();
        let metadata: AuthorizationMetadata = serde_json::from_value(serde_json::json!({
            "authorization_endpoint": "http://127.0.0.1:1/authorize",
            "token_endpoint": token_endpoint,
        }))
        .unwrap();
        manager.set_metadata(metadata);
        manager.configure_client_id("client").unwrap();

        let mut token = OAuthTokenResponse::new(
            AccessToken::new("stale".into()),
            BasicTokenType::Bearer,
            Default::default(),
        );
        token.set_expires_in(Some(&std::time::Duration::from_secs(1)));
        token.set_refresh_token(Some(oauth2::RefreshToken::new("refresh".into())));
        let store = InMemoryCredentialStore::new();
        rmcp::transport::auth::CredentialStore::save(
            &store,
            StoredCredentials::new("client".into(), Some(token), vec![], Some(0)),
        )
        .await
        .unwrap();
        manager.set_credential_store(store);
        RejectionAwareClient::new(AuthClient::new(reqwest::Client::new(), manager))
    }

    fn ping() -> ClientJsonRpcMessage {
        ClientJsonRpcMessage::request(
            ClientRequest::PingRequest(PingRequest::default()),
            rmcp::model::NumberOrString::Number(1),
        )
    }

    async fn post_error(client: &RejectionAwareClient<reqwest::Client>, uri: &str) -> String {
        let error = client
            .post_message(uri.into(), ping(), None, None, HashMap::new())
            .await
            .unwrap_err();
        match error {
            StreamableHttpError::Auth(auth) => format!("auth:{auth}"),
            other => format!("other:{other}"),
        }
    }

    fn rejected(error: &str) -> bool {
        let reason = error.strip_prefix("auth:OAuth token refresh failed: ");
        error == "auth:OAuth authorization required"
            || reason.is_some_and(is_definitive_refresh_rejection)
    }

    #[tokio::test]
    async fn refresh_invalid_grant_is_rejected_through_auth_client() {
        let token_app = Router::new().route(
            "/token",
            post(|| async {
                (
                    StatusCode::BAD_REQUEST,
                    [("content-type", "application/json")],
                    r#"{"error":"invalid_grant"}"#,
                )
            }),
        );
        let base = serve(token_app).await;
        let client = expired_client(&format!("{base}/token")).await;
        let error = post_error(&client, &format!("{base}/mcp")).await;
        assert!(rejected(&error), "{error}");
    }

    #[tokio::test]
    async fn refresh_network_failure_is_transient_through_auth_client() {
        let client = expired_client("http://127.0.0.1:1/token").await;
        let error = post_error(&client, "http://127.0.0.1:1/mcp").await;
        assert!(error.starts_with("auth:"), "{error}");
        assert!(!rejected(&error), "{error}");
    }

    #[tokio::test]
    async fn unchallenged_401_is_rejected_but_5xx_is_not() {
        let app = Router::new()
            .route(
                "/unauthorized",
                post(|| async { (StatusCode::UNAUTHORIZED, "Unauthorized") }),
            )
            .route(
                "/down",
                post(|| async { (StatusCode::SERVICE_UNAVAILABLE, "401 maintenance") }),
            );
        let base = serve(app).await;
        let client = fresh_token_client().await;

        let error = post_error(&client, &format!("{base}/unauthorized")).await;
        assert!(rejected(&error), "{error}");
        let error = post_error(&client, &format!("{base}/down")).await;
        assert!(error.starts_with("other:"), "{error}");
    }

    /// `AuthClient` with a still-valid token so requests reach the server.
    async fn fresh_token_client() -> RejectionAwareClient<reqwest::Client> {
        let mut manager = AuthorizationManager::new("http://127.0.0.1:1/mcp")
            .await
            .unwrap();
        let token = OAuthTokenResponse::new(
            AccessToken::new("fresh".into()),
            BasicTokenType::Bearer,
            Default::default(),
        );
        let store = InMemoryCredentialStore::new();
        rmcp::transport::auth::CredentialStore::save(
            &store,
            StoredCredentials::new("client".into(), Some(token), vec![], None),
        )
        .await
        .unwrap();
        manager.set_credential_store(store);
        RejectionAwareClient::new(AuthClient::new(reqwest::Client::new(), manager))
    }
}
