//! Gateway services
//!
//! Business logic services following SOLID principles:
//! - Each service has single responsibility (SRP)
//! - Services depend on abstractions (DIP)
//! - Open for extension, closed for modification (OCP)

mod authorization;
mod client_metadata_service;
mod discovery_rank;
mod event_emitter;
mod feature_set_resolver;
mod grant_service;
pub mod meta_tools;
mod notification_emitter;
mod prefix_cache;
pub mod prompt_discovery;
pub mod resource_discovery;
mod session_roots;
mod space_resolver;
pub mod tool_discovery;

pub use authorization::AuthorizationService;
pub use client_metadata_service::ClientMetadataService;
pub use discovery_rank::levenshtein_suggestions;
pub use event_emitter::EventEmitter;
pub use feature_set_resolver::{FeatureSetResolverService, ResolutionSource, ResolvedFeatureSet};
pub use grant_service::GrantService;
pub use meta_tools::{
    is_meta_tool, routing_as_invoke_backend, ApprovalBroker, ApprovalDecision, ApprovalPayload,
    ApprovalPublisher, ApprovalRequest, ApprovalScope, InvokeToolBackend, MetaToolRegistry,
    META_TOOL_APPROVAL_EVENT, MCPMUX_PREFIX,
};
pub use notification_emitter::NotificationEmitter;
pub use prefix_cache::PrefixCacheService;
pub use prompt_discovery::{PromptDetailLevel, PromptDiscoveryService, PromptIndexEntry};
pub use resource_discovery::{ResourceDetailLevel, ResourceDiscoveryService, ResourceIndexEntry};
pub use session_roots::SessionRootsRegistry;
pub use space_resolver::SpaceResolverService;
pub use tool_discovery::{DetailLevel, ToolDiscoveryService, ToolIndexEntry};
