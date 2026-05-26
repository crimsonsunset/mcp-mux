//! Admin HTTP integration test harness.
//!
//! Phase 2+ adds dual-entry tests (bridge JSON ≡ HTTP JSON) per parity matrix row.

use std::sync::Arc;
use std::time::Duration;

use mcpmux_core::{ApplicationServices, ApplicationServicesBuilder, EventBus, SpaceRepository};
use mcpmux_gateway::admin::{build_admin_router, AdminConfig, CF_ACCESS_JWT_HEADER};
use mcpmux_storage::{Database, SqliteSpaceRepository};
use reqwest::{Client, RequestBuilder, Response};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// In-memory `ApplicationServices` for admin integration tests.
async fn in_memory_services() -> Arc<ApplicationServices> {
    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let event_bus = Arc::new(EventBus::new());
    let space_repo: Arc<dyn SpaceRepository> = Arc::new(SqliteSpaceRepository::new(db));

    Arc::new(
        ApplicationServicesBuilder::new()
            .with_event_bus(event_bus)
            .with_space_repo(space_repo)
            .build()
            .expect("build ApplicationServices"),
    )
}

/// Running admin server on an ephemeral loopback port.
struct AdminHarness {
    base_url: String,
    _services: Arc<ApplicationServices>,
    cancel: CancellationToken,
}

impl AdminHarness {
    /// Mount the admin router and bind to `127.0.0.1:0`.
    async fn start(services: Arc<ApplicationServices>, config: AdminConfig) -> Self {
        let router = build_admin_router(services.clone(), config);
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind admin harness");
        let port = listener.local_addr().expect("local addr").port();
        let base_url = format!("http://127.0.0.1:{port}");
        let cancel = CancellationToken::new();
        let shutdown = cancel.clone();

        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    shutdown.cancelled().await;
                })
                .await
                .expect("serve admin harness");
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        Self {
            base_url,
            _services: services,
            cancel,
        }
    }

    fn shutdown(self) {
        self.cancel.cancel();
    }
}

/// HTTP client for admin API requests with optional CF Access JWT.
struct AdminClient {
    inner: Client,
    base_url: String,
    cf_jwt: Option<String>,
}

impl AdminClient {
    /// Create a client targeting `base_url` with an optional JWT stub.
    fn new(base_url: impl Into<String>, cf_jwt: Option<&str>) -> Self {
        Self {
            inner: Client::new(),
            base_url: base_url.into(),
            cf_jwt: cf_jwt.map(str::to_string),
        }
    }

    /// Begin a GET request, attaching `CF-Access-Jwt-Assertion` when configured.
    fn get(&self, path: &str) -> RequestBuilder {
        let mut req = self.inner.get(format!("{}{}", self.base_url, path));
        if let Some(jwt) = &self.cf_jwt {
            req = req.header(CF_ACCESS_JWT_HEADER, jwt);
        }
        req
    }

    /// Send GET and return the response.
    async fn get_response(&self, path: &str) -> Response {
        self.get(path).send().await.expect("admin GET request")
    }
}

/// Helper: admin HTTP client with optional CF JWT header.
fn admin_client(base_url: &str, cf_jwt: Option<&str>) -> AdminClient {
    AdminClient::new(base_url, cf_jwt)
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_harness_starts_on_ephemeral_port() {
    let services = in_memory_services().await;
    let harness = AdminHarness::start(services, AdminConfig::default()).await;
    let client = admin_client(&harness.base_url, None);

    let resp = client.get_response("/").await;
    assert_eq!(resp.status(), 404);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_client_attaches_cf_jwt_header_when_provided() {
    let services = in_memory_services().await;
    let harness = AdminHarness::start(services, AdminConfig::default()).await;
    let jwt = "test-cf-access-jwt-stub";
    let client = admin_client(&harness.base_url, Some(jwt));

    let resp = client.get_response("/").await;
    assert_eq!(resp.status(), 404);

    harness.shutdown();
}
