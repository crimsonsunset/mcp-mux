//! Whether inbound MCP auth is off, as one rule for startup, Settings, and
//! live exposure changes.
//!
//! An explicit `gateway.auth_disabled` choice wins. With none stored, auth is
//! off only while just this machine can reach the gateway (loopback bind, no
//! public URL). Anything that can't be read reliably counts as "auth required".

use std::sync::Arc;

use mcpmux_core::AppSettingsRepository;
use tokio::sync::RwLock;

use crate::server::GatewayState;

pub const AUTH_DISABLED_KEY: &str = "gateway.auth_disabled";
pub const NETWORK_ACCESS_KEY: &str = "gateway.network_access_enabled";
pub const PUBLIC_BASE_URL_KEY: &str = "gateway.public_base_url";

/// Effective "inbound auth is off" value for the persisted settings.
pub async fn effective_auth_disabled(settings: &Arc<dyn AppSettingsRepository>) -> bool {
    let Ok(stored) = settings.get(AUTH_DISABLED_KEY).await else {
        return false;
    };
    if let Some(value) = stored {
        return value == "true";
    }
    let (Ok(network), Ok(public_url)) = (
        settings.get(NETWORK_ACCESS_KEY).await,
        settings.get(PUBLIC_BASE_URL_KEY).await,
    ) else {
        return false;
    };
    network.as_deref() != Some("true") && public_url.is_none_or(|url| url.trim().is_empty())
}

/// Re-apply the effective value to a running gateway after exposure changed
/// (e.g. a public URL was set live). An explicit stored choice is left alone.
pub async fn refresh_live_auth_default(
    settings: &Arc<dyn AppSettingsRepository>,
    gateway_state: &Arc<RwLock<GatewayState>>,
) {
    if matches!(settings.get(AUTH_DISABLED_KEY).await, Ok(None)) {
        let disabled = effective_auth_disabled(settings).await;
        gateway_state.write().await.set_auth_disabled(disabled);
    }
}
