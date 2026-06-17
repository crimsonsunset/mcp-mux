//! `mcpmux_bind_current_workspace` — persistently layer a FeatureSet onto a workspace binding.

use async_trait::async_trait;
use mcpmux_core::{normalize_workspace_root, WorkspaceBinding};
use rmcp::model::CallToolResult;
use serde_json::{json, Value};
use tracing::info;

use super::meta_tool_common::{
    caller_space_id, emit_tools_list_changed, emit_workspace_binding_changed, parse_uuid_arg,
    text_result, with_approval,
};
use super::registry::{MetaTool, MetaToolCall, MetaToolError};

pub struct BindCurrentWorkspaceTool;

#[async_trait]
impl MetaTool for BindCurrentWorkspaceTool {
    fn name(&self) -> &'static str {
        "mcpmux_bind_current_workspace"
    }

    fn description(&self) -> &'static str {
        "Canonical activation path: persistently append an existing FeatureSet \
         onto the caller's workspace binding (layers with existing bundles, \
         deduped). Use after mcpmux_search_tools (include_inactive: true) or \
         mcpmux_list_feature_sets to obtain feature_set_id. Every future \
         connection reporting the same root inherits the binding. Requires \
         approval; the client MUST have declared MCP roots."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["feature_set_id"],
            "properties": {
                "feature_set_id": { "type": "string" }
            }
        })
    }

    fn is_write(&self) -> bool {
        true
    }

    async fn call(&self, call: MetaToolCall<'_>) -> Result<CallToolResult, MetaToolError> {
        let fs_id = parse_uuid_arg(&call.args, "feature_set_id")?;

        let space_id = caller_space_id(&call).await?;
        let roots = call
            .session_id
            .and_then(|sid| call.ctx.session_roots.get(sid))
            .unwrap_or_default();
        let root = roots.into_iter().next().ok_or_else(|| {
            MetaToolError::InvalidArgument(
                "caller did not report any MCP roots; cannot bind — \
                 call mcpmux_set_workspace_root first to declare your workspace path, \
                 then retry mcpmux_bind_current_workspace"
                    .into(),
            )
        })?;
        let normalized = normalize_workspace_root(&root);

        let fs_name = call
            .ctx
            .feature_set_repo
            .get(&fs_id.to_string())
            .await?
            .map(|fs| fs.name)
            .unwrap_or_else(|| fs_id.to_string());

        let binding_repo = call.ctx.binding_repo.clone();
        let fs_id_str = fs_id.to_string();
        let caller_client_id = call.client_id.to_string();

        // Dedup before consent: repeat binds must not re-prompt the user.
        if let Some(existing) = binding_repo
            .find_longest_prefix_match(&space_id, Some(&caller_client_id), std::slice::from_ref(&normalized))
            .await?
        {
            if existing.feature_set_ids.iter().any(|id| id == &fs_id_str) {
                return Ok(text_result(json!({
                    "ok": true,
                    "binding_id": existing.id,
                    "workspace_root": normalized,
                    "feature_set_id": fs_id,
                    "feature_set_ids": existing.feature_set_ids,
                    "already_bound": true,
                })));
            }
        }

        let summary = format!(
            "Append FeatureSet '{fs_name}' to workspace '{normalized}' binding \
             for client '{caller_client_id}' (existing bundles preserved)."
        );

        let event_tx = call.ctx.domain_event_tx.clone();
        with_approval(
            &call,
            "mcpmux_bind_current_workspace",
            summary,
            None,
            true,
            call.args.clone(),
            || async move {
                let fs_id_str = fs_id.to_string();
                let existing = binding_repo
                    .find_longest_prefix_match(
                        &space_id,
                        Some(&caller_client_id),
                        std::slice::from_ref(&normalized),
                    )
                    .await?;

                let (binding_id, feature_set_ids, already_bound) = if let Some(mut binding) =
                    existing
                {
                    binding.space_id = space_id;
                    let already_bound = binding.feature_set_ids.iter().any(|id| id == &fs_id_str);
                    if !already_bound {
                        binding.feature_set_ids.push(fs_id_str.clone());
                        binding.updated_at = chrono::Utc::now();
                        binding_repo.update(&binding).await?;
                        emit_workspace_binding_changed(&event_tx, space_id, &normalized);
                    }
                    info!(
                        %space_id,
                        client_id = %caller_client_id,
                        binding_id = %binding.id,
                        workspace_root = %normalized,
                        feature_set_id = %fs_id,
                        already_bound,
                        feature_set_count = binding.feature_set_ids.len(),
                        "[meta_tools] bind_current_workspace updated existing scoped binding",
                    );
                    (binding.id, binding.feature_set_ids.clone(), already_bound)
                } else {
                    let binding = WorkspaceBinding::new_scoped_multi(
                        normalized.clone(),
                        space_id,
                        Some(caller_client_id.clone()),
                        vec![fs_id_str.clone()],
                    );
                    let binding_id = binding.id;
                    let feature_set_ids = binding.feature_set_ids.clone();
                    binding_repo.create(&binding).await?;
                    info!(
                        %space_id,
                        client_id = %caller_client_id,
                        binding_id = %binding_id,
                        workspace_root = %normalized,
                        feature_set_id = %fs_id,
                        "[meta_tools] bind_current_workspace created scoped binding",
                    );
                    (binding_id, feature_set_ids, false)
                };

                emit_tools_list_changed(&event_tx, space_id);
                Ok(text_result(json!({
                    "ok": true,
                    "binding_id": binding_id,
                    "workspace_root": normalized,
                    "feature_set_id": fs_id,
                    "feature_set_ids": feature_set_ids,
                    "already_bound": already_bound,
                })))
            },
        )
        .await
    }
}
