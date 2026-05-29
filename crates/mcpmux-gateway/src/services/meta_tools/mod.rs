//! Self-management meta tools (`mcpmux_*`).
//!
//! A small built-in toolset exposed by the gateway alongside the filtered
//! backend tools. Lets connected LLMs introspect the currently resolved
//! FeatureSet, see what tools exist unfiltered, and — gated by user
//! approval — reshape their own session's toolset (pin, create FS, bind
//! workspace, flip the Space's active FS).
//!
//! Design: the write tools are the token-savings feature. When a project
//! only needs 10 of 80 connected tools, the LLM can call
//! `mcpmux_pin_this_session` after reviewing the workspace, and the next
//! `tools/list` returns only the 10. Existing `tools/list_changed`
//! notification plumbing lands the reduced set in-session.
//!
//! Security: every write tool routes through [`approval::ApprovalBroker`]
//! which pops a native desktop dialog showing the concrete tool-list diff
//! before allowing the change. Headless gateways return `approval_required`.
//! Reads are unmetered.
//!
//! Namespace: all meta tools have names starting with `MCPMUX_PREFIX`
//! (`mcpmux_`) so the handler can route them before feature-set filtering.

pub mod approval;
pub mod diagnose;
pub mod diff;
pub mod disclosure;
pub mod disclosure_backend;
pub mod invoke;
pub mod invoke_backend;
mod registry;
mod tools;

pub use approval::{
    ApprovalBroker, ApprovalDecision, ApprovalPayload, ApprovalPublisher, ApprovalRequest,
    ApprovalScope,
};
pub use diff::ToolDiff;
pub use disclosure_backend::{pool_as_disclosure_backend, DisclosureBackend};
pub use invoke_backend::{routing_as_invoke_backend, InvokeToolBackend};
pub use registry::{MetaToolContext, MetaToolError, MetaToolRegistry, META_TOOLS_ENABLED_KEY};

use crate::services::ToolDiscoveryService;

/// Every built-in tool's name must start with this prefix so the handler
/// can intercept it before routing to backend servers.
pub const MCPMUX_PREFIX: &str = "mcpmux_";

/// Convenience: is this tool name one of ours?
pub fn is_meta_tool(name: &str) -> bool {
    name.starts_with(MCPMUX_PREFIX)
}

/// Factory wiring a fully-configured registry with every default tool.
///
/// Callers (ServiceContainer) construct one of these at gateway startup
/// and clone the Arc freely.
#[allow(clippy::too_many_arguments)]
pub fn build_default_registry(
    client_repo: std::sync::Arc<dyn mcpmux_core::InboundMcpClientRepository>,
    space_repo: std::sync::Arc<dyn mcpmux_core::SpaceRepository>,
    feature_set_repo: std::sync::Arc<dyn mcpmux_core::FeatureSetRepository>,
    binding_repo: std::sync::Arc<dyn mcpmux_core::WorkspaceBindingRepository>,
    server_feature_repo: std::sync::Arc<dyn mcpmux_core::ServerFeatureRepository>,
    installed_server_repo: std::sync::Arc<dyn mcpmux_core::InstalledServerRepository>,
    resolver: std::sync::Arc<crate::services::FeatureSetResolverService>,
    feature_service: std::sync::Arc<crate::pool::FeatureService>,
    invoke_backend: Option<std::sync::Arc<dyn invoke_backend::InvokeToolBackend>>,
    disclosure_backend: Option<std::sync::Arc<dyn disclosure_backend::DisclosureBackend>>,
    session_roots: std::sync::Arc<crate::services::SessionRootsRegistry>,
    approval_broker: std::sync::Arc<ApprovalBroker>,
    domain_event_tx: tokio::sync::broadcast::Sender<mcpmux_core::DomainEvent>,
    settings_repo: Option<std::sync::Arc<dyn mcpmux_core::AppSettingsRepository>>,
    server_manager: std::sync::Arc<crate::pool::ServerManager>,
    log_manager: std::sync::Arc<mcpmux_core::ServerLogManager>,
) -> std::sync::Arc<MetaToolRegistry> {
    let tool_discovery =
        std::sync::Arc::new(ToolDiscoveryService::new(server_feature_repo.clone()));
    let resource_discovery = std::sync::Arc::new(crate::services::ResourceDiscoveryService::new(
        server_feature_repo.clone(),
    ));
    let prompt_discovery = std::sync::Arc::new(crate::services::PromptDiscoveryService::new(
        server_feature_repo.clone(),
    ));
    let ctx = MetaToolContext {
        client_repo,
        space_repo,
        feature_set_repo,
        binding_repo,
        server_feature_repo,
        installed_server_repo,
        resolver,
        feature_service,
        invoke_backend,
        tool_discovery,
        resource_discovery,
        prompt_discovery,
        disclosure_backend,
        session_roots,
        approval_broker,
        domain_event_tx,
        settings_repo,
        server_manager,
        log_manager,
    };

    let mut registry = MetaToolRegistry::new(ctx);
    // Reads — no approval needed.
    registry.register(Box::new(tools::ListFeatureSetsTool));
    registry.register(Box::new(tools::ListServersTool));
    registry.register(Box::new(tools::SearchToolsTool));
    registry.register(Box::new(tools::GetToolSchemaTool));
    registry.register(Box::new(diagnose::DiagnoseServerTool));
    registry.register(Box::new(invoke::InvokeToolTool));
    registry.register(Box::new(disclosure::SearchResourcesTool));
    registry.register(Box::new(disclosure::ReadResourceTool));
    registry.register(Box::new(disclosure::SearchPromptsTool));
    registry.register(Box::new(disclosure::FetchPromptTool));
    // Writes — gated by ApprovalBroker (bind-only; humans author bundles in UI).
    registry.register(Box::new(tools::BindCurrentWorkspaceTool));
    std::sync::Arc::new(registry)
}
