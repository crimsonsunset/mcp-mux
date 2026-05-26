//! Integration tests for McpMux core flows
//!
//! Tests the complete inbound/outbound MCP flows:
//! - Feature grant resolution (Space → FeatureSet → Features)
//! - Feature routing (qualified names, prefix resolution)
//! - MCP request handling (tools, resources, prompts)
//!
//! NOTE: Authorization tests that require InboundClientRepository
//! are in the database tests since they need the real SQLite implementation.

mod admin_api;
mod admin_api_oauth;
mod admin_api_regression;
mod admin_api_write;
mod admin_sse_events;
mod command_bridge_space;
mod feature_routing;
mod feature_set_resolver;
mod mcp_flows;
mod meta_gateway_invoke;
mod meta_tools;
mod server_clone;
mod workspace_binding_events;
