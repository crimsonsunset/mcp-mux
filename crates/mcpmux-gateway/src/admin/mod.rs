//! Web admin HTTP server (REST + static SPA).
//!
//! Serves the built React admin UI and `/api/v1/*` REST endpoints on a
//! separate loopback port (default `45819`), gated by Cloudflare Access
//! when configured.

mod config;
pub mod bridge_context;
pub mod command_bridge;
pub mod event_hub;
mod handlers;
mod live_runtime;
mod middleware;
mod router;
pub mod runtime;
mod server;
pub mod ui_events;
pub mod write_runtime;

pub use config::{AdminConfig, CF_ACCESS_JWT_HEADER, DEFAULT_ADMIN_PORT};
pub use bridge_context::AdminBridgeCtx;
pub use event_hub::AdminEventHub;
pub use ui_events::{map_domain_event_to_ui, AdminUiEventBus, UiEvent};
pub use middleware::{CfAccessError, CfAccessValidator, CSRF_HEADER, new_csrf_token_store};
#[cfg(any(test, feature = "test-utils"))]
pub use handlers::error::format_bridge_error_message;
pub use runtime::GatewayRuntime;
#[cfg(any(test, feature = "test-utils"))]
pub use runtime::StubGatewayRuntime;
pub use live_runtime::LiveGatewayRuntime;
pub use write_runtime::GatewayWriteRuntime;
#[cfg(any(test, feature = "test-utils"))]
pub use write_runtime::StubGatewayWriteRuntime;
pub use router::{build_admin_router, AdminState};
pub use server::{AdminServer, AdminServerHandle};

#[cfg(any(test, feature = "test-utils"))]
#[doc(hidden)]
pub use middleware::{test_valid_jwt, test_validator};
