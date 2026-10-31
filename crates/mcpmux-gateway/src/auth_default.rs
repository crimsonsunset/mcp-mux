//! Whether inbound MCP auth is off, as one rule for startup, Settings, and
//! live exposure changes.
//!
//! Auth is always required while the gateway is reachable from other machines
//! (network bind or public URL), whatever was chosen earlier. While only this
//! machine can reach it, an explicit `gateway.auth_disabled` choice wins, and
//! with none stored auth is off. Anything that can't be read reliably counts as
//! "auth required".

use std::sync::Arc;

use mcpmux_core::AppSettingsRepository;
use tokio::sync::RwLock;

use crate::server::GatewayState;

pub const AUTH_DISABLED_KEY: &str = "gateway.auth_disabled";
pub const NETWORK_ACCESS_KEY: &str = "gateway.network_access_enabled";
pub const PUBLIC_BASE_URL_KEY: &str = "gateway.public_base_url";

/// Effective "inbound auth is off" value for the persisted settings.
pub async fn effective_auth_disabled(settings: &Arc<dyn AppSettingsRepository>) -> bool {
    let (Ok(stored), Ok(network), Ok(public_url)) = (
        settings.get(AUTH_DISABLED_KEY).await,
        settings.get(NETWORK_ACCESS_KEY).await,
        settings.get(PUBLIC_BASE_URL_KEY).await,
    ) else {
        return false;
    };
    let exposed =
        network.as_deref() == Some("true") || public_url.is_some_and(|url| !url.trim().is_empty());
    if exposed {
        return false;
    }
    stored.is_none_or(|value| value == "true")
}

/// Re-apply the effective value to a running gateway after exposure changed
/// (e.g. a public URL was set live), so an explicit "off" does not outlive the
/// exposure that now forbids it.
pub async fn refresh_live_auth_default(
    settings: &Arc<dyn AppSettingsRepository>,
    gateway_state: &Arc<RwLock<GatewayState>>,
) {
    let disabled = effective_auth_disabled(settings).await;
    gateway_state.write().await.set_auth_disabled(disabled);
}
