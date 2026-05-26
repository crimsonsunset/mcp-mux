//! Admin HTTP integration tests.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use mcpmux_core::{ApplicationServices, ApplicationServicesBuilder, EventBus, SpaceRepository};
use mcpmux_gateway::admin::{
    build_admin_router, test_valid_jwt, test_validator, AdminConfig, AdminState,
    CF_ACCESS_JWT_HEADER,
};
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
    async fn start(config: AdminConfig, gateway_running: bool) -> Self {
        let services = in_memory_services().await;
        let gateway_flag = Arc::new(AtomicBool::new(gateway_running));
        let cf_validator = if config.trust_cf_access {
            config
                .cf_validator_override
                .clone()
                .or_else(|| Some(test_validator()))
        } else {
            None
        };

        let state = AdminState {
            services: services.clone(),
            config: config.clone(),
            gateway_running: gateway_flag,
            frontend_dist: std::path::PathBuf::from("/nonexistent"),
            cf_validator,
        };
        let router = build_admin_router(state);

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

#[tokio::test(flavor = "multi_thread")]
async fn health_returns_200_with_valid_jwt_stub() {
    let mut config = AdminConfig {
        trust_cf_access: true,
        ..AdminConfig::default()
    };
    config.cf_validator_override = Some(test_validator());

    let harness = AdminHarness::start(config, true).await;
    let client = AdminClient::new(&harness.base_url, Some(&test_valid_jwt()));

    let resp = client.get_response("/api/v1/health").await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.expect("health json");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["gateway_running"], true);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn health_returns_401_when_cf_access_enabled_and_no_jwt() {
    let mut config = AdminConfig {
        trust_cf_access: true,
        ..AdminConfig::default()
    };
    config.cf_validator_override = Some(test_validator());

    let harness = AdminHarness::start(config, false).await;
    let client = AdminClient::new(&harness.base_url, None);

    let resp = client.get_response("/api/v1/health").await;
    assert_eq!(resp.status(), 401);

    harness.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn health_returns_200_when_cf_access_disabled() {
    let harness = AdminHarness::start(AdminConfig::default(), false).await;
    let client = AdminClient::new(&harness.base_url, None);

    let resp = client.get_response("/api/v1/health").await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.expect("health json");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["gateway_running"], false);

    harness.shutdown();
}
