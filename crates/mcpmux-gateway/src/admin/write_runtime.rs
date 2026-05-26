//! Runtime adapter for gateway-dependent admin write operations.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Async runtime adapter for writes that depend on live gateway / desktop state.
#[async_trait]
pub trait GatewayWriteRuntime: Send + Sync {
    async fn start_gateway(
        &self,
        port: Option<u16>,
        allow_dynamic_fallback: Option<bool>,
    ) -> Result<Value>;
    async fn stop_gateway(&self) -> Result<Value>;
    async fn restart_gateway(
        &self,
        port: Option<u16>,
        allow_dynamic_fallback: Option<bool>,
    ) -> Result<Value>;
    async fn disconnect_server(
        &self,
        server_id: String,
        space_id: String,
        logout: Option<bool>,
    ) -> Result<Value>;
    async fn connect_all_enabled_servers(&self) -> Result<Value>;
    async fn refresh_oauth_tokens_on_startup(&self) -> Result<Value>;
    async fn set_gateway_port(&self, port: u16) -> Result<Value>;
    async fn enable_server_v2(&self, space_id: String, server_id: String) -> Result<Value>;
    async fn disable_server_v2(&self, space_id: String, server_id: String) -> Result<Value>;
    async fn start_auth_v2(&self, space_id: String, server_id: String) -> Result<Value>;
    async fn cancel_auth_v2(&self, space_id: String, server_id: String) -> Result<Value>;
    async fn retry_connection(&self, space_id: String, server_id: String) -> Result<Value>;
    async fn logout_server(&self, space_id: String, server_id: String) -> Result<Value>;
    async fn clear_session_overrides(&self, session_id: String) -> Result<Value>;
    async fn respond_to_meta_tool_approval(
        &self,
        request_id: String,
        client_id: String,
        tool_name: String,
        decision: String,
    ) -> Result<Value>;
    async fn revoke_meta_tool_grant(&self, client_id: String, tool_name: String) -> Result<Value>;
    async fn update_oauth_client(
        &self,
        client_id: String,
        client_alias: Option<String>,
    ) -> Result<Value>;
    async fn delete_oauth_client(&self, client_id: String) -> Result<Value>;
    async fn grant_oauth_client_feature_set(
        &self,
        client_id: String,
        space_id: String,
        feature_set_id: String,
    ) -> Result<Value>;
    async fn revoke_oauth_client_feature_set(
        &self,
        client_id: String,
        space_id: String,
        feature_set_id: String,
    ) -> Result<Value>;
}

fn gateway_not_running() -> anyhow::Error {
    anyhow::anyhow!("Gateway not running")
}

/// Test/default write runtime — gateway ops fail; port persist succeeds as no-op.
pub struct StubGatewayWriteRuntime {
    pub gateway_port_service: Option<std::sync::Arc<mcpmux_core::GatewayPortService>>,
}

impl Default for StubGatewayWriteRuntime {
    fn default() -> Self {
        Self {
            gateway_port_service: None,
        }
    }
}

#[async_trait]
impl GatewayWriteRuntime for StubGatewayWriteRuntime {
    async fn start_gateway(
        &self,
        _port: Option<u16>,
        _allow_dynamic_fallback: Option<bool>,
    ) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn stop_gateway(&self) -> Result<Value> {
        Ok(serde_json::json!({ "ok": true }))
    }

    async fn restart_gateway(
        &self,
        _port: Option<u16>,
        _allow_dynamic_fallback: Option<bool>,
    ) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn disconnect_server(
        &self,
        _server_id: String,
        _space_id: String,
        _logout: Option<bool>,
    ) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn connect_all_enabled_servers(&self) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn refresh_oauth_tokens_on_startup(&self) -> Result<Value> {
        Ok(serde_json::json!({
            "servers_checked": 0,
            "tokens_refreshed": 0,
            "refresh_failed": 0,
        }))
    }

    async fn set_gateway_port(&self, port: u16) -> Result<Value> {
        if port < 1024 {
            return Err(anyhow::anyhow!(
                "Port {port} is in the privileged range (≤ 1023). Choose a port between 1024 and 65535."
            ));
        }
        if let Some(ref svc) = self.gateway_port_service {
            svc.save_port(port).await?;
        }
        Ok(serde_json::json!({ "ok": true }))
    }

    async fn enable_server_v2(&self, _space_id: String, _server_id: String) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn disable_server_v2(&self, _space_id: String, _server_id: String) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn start_auth_v2(&self, _space_id: String, _server_id: String) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn cancel_auth_v2(&self, _space_id: String, _server_id: String) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn retry_connection(&self, _space_id: String, _server_id: String) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn logout_server(&self, _space_id: String, _server_id: String) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn clear_session_overrides(&self, _session_id: String) -> Result<Value> {
        Ok(serde_json::json!({ "ok": true }))
    }

    async fn respond_to_meta_tool_approval(
        &self,
        _request_id: String,
        _client_id: String,
        _tool_name: String,
        _decision: String,
    ) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn revoke_meta_tool_grant(
        &self,
        _client_id: String,
        _tool_name: String,
    ) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn update_oauth_client(
        &self,
        _client_id: String,
        _client_alias: Option<String>,
    ) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn delete_oauth_client(&self, _client_id: String) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn grant_oauth_client_feature_set(
        &self,
        _client_id: String,
        _space_id: String,
        _feature_set_id: String,
    ) -> Result<Value> {
        Err(gateway_not_running())
    }

    async fn revoke_oauth_client_feature_set(
        &self,
        _client_id: String,
        _space_id: String,
        _feature_set_id: String,
    ) -> Result<Value> {
        Err(gateway_not_running())
    }
}
