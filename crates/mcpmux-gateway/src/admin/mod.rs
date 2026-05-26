//! Web admin HTTP server (REST + static SPA).
//!
//! Phase 2 adds health, static file serving, and CF Access middleware.

mod config;
mod router;

pub use config::{AdminConfig, CF_ACCESS_JWT_HEADER, DEFAULT_ADMIN_PORT};
pub use router::{build_admin_router, AdminState};
