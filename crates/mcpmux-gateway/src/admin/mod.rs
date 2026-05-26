//! Web admin HTTP server (REST + static SPA).
//!
//! Serves the built React admin UI and `/api/v1/*` REST endpoints on a
//! separate loopback port (default `45819`), gated by Cloudflare Access
//! when configured.

mod config;
mod handlers;
mod middleware;
mod router;
mod server;

pub use config::{AdminConfig, CF_ACCESS_JWT_HEADER, DEFAULT_ADMIN_PORT};
pub use middleware::{CfAccessError, CfAccessValidator};
pub use router::{build_admin_router, AdminState};
pub use server::{AdminServer, AdminServerHandle};

#[doc(hidden)]
pub use middleware::{test_valid_jwt, test_validator};
