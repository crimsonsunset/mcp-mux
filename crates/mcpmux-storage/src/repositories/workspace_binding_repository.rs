//! SQLite implementation of WorkspaceBindingRepository.
//!
//! See the trait docs on [`mcpmux_core::WorkspaceBindingRepository`] for the
//! semantics of "longest-prefix-wins" matching — paths are expected to be
//! already normalized by [`mcpmux_core::normalize_workspace_root`] before being
//! stored or queried.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mcpmux_core::{longest_prefix_match, WorkspaceBinding, WorkspaceBindingRepository};
use rusqlite::{params, OptionalExtension};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::Database;

pub struct SqliteWorkspaceBindingRepository {
    db: Arc<Mutex<Database>>,
}

impl SqliteWorkspaceBindingRepository {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    fn parse_datetime(s: &str) -> DateTime<Utc> {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return dt.with_timezone(&Utc);
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return dt.and_utc();
        }
        Utc::now()
    }

    fn row_to_binding(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceBinding> {
        let id_str: String = row.get(0)?;
        let space_id_str: String = row.get(1)?;
        let fs_id_str: String = row.get(3)?;
        Ok(WorkspaceBinding {
            id: id_str.parse().unwrap_or_else(|_| Uuid::new_v4()),
            space_id: space_id_str.parse().unwrap_or_else(|_| Uuid::new_v4()),
            workspace_root: row.get(2)?,
            feature_set_id: fs_id_str.parse().unwrap_or_else(|_| Uuid::new_v4()),
            created_at: Self::parse_datetime(&row.get::<_, String>(4)?),
            updated_at: Self::parse_datetime(&row.get::<_, String>(5)?),
        })
    }
}

#[async_trait]
impl WorkspaceBindingRepository for SqliteWorkspaceBindingRepository {
    async fn list(&self) -> Result<Vec<WorkspaceBinding>> {
        let db = self.db.lock().await;
        let conn = db.connection();

        let mut stmt = conn.prepare(
            "SELECT id, space_id, workspace_root, feature_set_id, created_at, updated_at
             FROM workspace_bindings
             ORDER BY space_id, workspace_root",
        )?;

        let bindings = stmt
            .query_map([], Self::row_to_binding)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(bindings)
    }

    async fn list_for_space(&self, space_id: &Uuid) -> Result<Vec<WorkspaceBinding>> {
        let db = self.db.lock().await;
        let conn = db.connection();

        let mut stmt = conn.prepare(
            "SELECT id, space_id, workspace_root, feature_set_id, created_at, updated_at
             FROM workspace_bindings
             WHERE space_id = ?
             ORDER BY workspace_root",
        )?;

        let bindings = stmt
            .query_map(params![space_id.to_string()], Self::row_to_binding)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(bindings)
    }

    async fn get(&self, id: &Uuid) -> Result<Option<WorkspaceBinding>> {
        let db = self.db.lock().await;
        let conn = db.connection();

        let mut stmt = conn.prepare(
            "SELECT id, space_id, workspace_root, feature_set_id, created_at, updated_at
             FROM workspace_bindings
             WHERE id = ?",
        )?;
        let binding = stmt
            .query_row(params![id.to_string()], Self::row_to_binding)
            .optional()?;
        Ok(binding)
    }

    async fn create(&self, binding: &WorkspaceBinding) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.connection();

        conn.execute(
            "INSERT INTO workspace_bindings (id, space_id, workspace_root, feature_set_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                binding.id.to_string(),
                binding.space_id.to_string(),
                binding.workspace_root,
                binding.feature_set_id.to_string(),
                binding.created_at.to_rfc3339(),
                binding.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    async fn update(&self, binding: &WorkspaceBinding) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.connection();

        let rows_affected = conn.execute(
            "UPDATE workspace_bindings
             SET space_id = ?2, workspace_root = ?3, feature_set_id = ?4, updated_at = ?5
             WHERE id = ?1",
            params![
                binding.id.to_string(),
                binding.space_id.to_string(),
                binding.workspace_root,
                binding.feature_set_id.to_string(),
                binding.updated_at.to_rfc3339(),
            ],
        )?;

        if rows_affected == 0 {
            anyhow::bail!("WorkspaceBinding not found: {}", binding.id);
        }

        Ok(())
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let db = self.db.lock().await;
        let conn = db.connection();

        conn.execute(
            "DELETE FROM workspace_bindings WHERE id = ?",
            params![id.to_string()],
        )?;

        Ok(())
    }

    async fn find_longest_prefix_match(
        &self,
        space_id: &Uuid,
        candidate_roots: &[String],
    ) -> Result<Option<WorkspaceBinding>> {
        if candidate_roots.is_empty() {
            return Ok(None);
        }

        // Load all bindings for this space up-front. In practice a Space holds
        // O(10) bindings, so SQL-side prefix matching is unnecessary complexity.
        let bindings = self.list_for_space(space_id).await?;
        if bindings.is_empty() {
            return Ok(None);
        }

        let candidate_strings: Vec<&str> =
            bindings.iter().map(|b| b.workspace_root.as_str()).collect();

        // For each reported root, find the longest binding prefix.
        // Across multiple roots, pick whichever winning prefix is longest.
        let mut best: Option<&WorkspaceBinding> = None;
        for root in candidate_roots {
            if let Some(winner) = longest_prefix_match(root, candidate_strings.iter().copied()) {
                let winning = bindings
                    .iter()
                    .find(|b| b.workspace_root == winner)
                    .expect("candidate came from bindings");
                if best
                    .map(|b| winning.workspace_root.len() > b.workspace_root.len())
                    .unwrap_or(true)
                {
                    best = Some(winning);
                }
            }
        }

        Ok(best.cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_repo() -> (SqliteWorkspaceBindingRepository, Uuid, Uuid) {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let repo = SqliteWorkspaceBindingRepository::new(db.clone());
        let default_space = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        // The default space has a default FS seeded by migration 001; use it.
        let fs_id = Uuid::parse_str(
            format!("fs_default_{}", default_space).trim_start_matches("fs_default_"),
        )
        .unwrap_or_else(|_| Uuid::new_v4());
        // The migration inserts a feature_set with id "fs_default_..." (not a UUID);
        // for test purposes create a custom FS we control.
        {
            let db_guard = db.lock().await;
            let now = Utc::now().to_rfc3339();
            db_guard
                .connection()
                .execute(
                    "INSERT INTO feature_sets (id, name, feature_set_type, space_id, is_builtin, created_at, updated_at)
                     VALUES (?1, 'Test FS', 'custom', ?2, 0, ?3, ?3)",
                    params![fs_id.to_string(), default_space.to_string(), now],
                )
                .unwrap();
        }
        (repo, default_space, fs_id)
    }

    #[tokio::test]
    async fn test_crud_and_prefix_match() {
        let (repo, space_id, fs_id) = make_repo().await;

        #[cfg(windows)]
        let (root_parent, root_child) = ("d:\\projects", "d:\\projects\\foo");
        #[cfg(not(windows))]
        let (root_parent, root_child) = ("/home/user/projects", "/home/user/projects/foo");

        let parent = WorkspaceBinding::new(space_id, root_parent, fs_id);
        let child = WorkspaceBinding::new(space_id, root_child, fs_id);
        repo.create(&parent).await.unwrap();
        repo.create(&child).await.unwrap();

        let all = repo.list_for_space(&space_id).await.unwrap();
        assert_eq!(all.len(), 2);

        // Exact match on child
        let found = repo
            .find_longest_prefix_match(&space_id, &[root_child.to_string()])
            .await
            .unwrap();
        assert_eq!(found.unwrap().workspace_root, root_child);

        // Deeper path should still hit child (longest prefix)
        #[cfg(windows)]
        let deeper = "d:\\projects\\foo\\src";
        #[cfg(not(windows))]
        let deeper = "/home/user/projects/foo/src";
        let found = repo
            .find_longest_prefix_match(&space_id, &[deeper.to_string()])
            .await
            .unwrap();
        assert_eq!(found.unwrap().workspace_root, root_child);

        // Empty candidates returns None
        let found = repo
            .find_longest_prefix_match(&space_id, &[])
            .await
            .unwrap();
        assert!(found.is_none());
    }
}
