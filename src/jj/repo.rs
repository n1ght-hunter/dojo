use anyhow::{Context, Result};
use jj_cli::config::{ConfigEnv, config_from_environment, default_config_layers};
use jj_lib::backend::CommitId;
use jj_lib::commit::Commit;
use jj_lib::config::{ConfigLayer, ConfigSource};
use jj_lib::object_id::ObjectId;
use jj_lib::repo::{ReadonlyRepo, Repo, StoreFactories};
use jj_lib::settings::UserSettings;
use jj_lib::workspace::{Workspace, default_working_copy_factories};
use std::path::Path;
use std::sync::Arc;

/// Information about a single commit for display
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub commit_id: String,
    pub change_id: String,
    pub description: String,
    pub author: String,
    pub timestamp: chrono::DateTime<chrono::FixedOffset>,
    pub parent_ids: Vec<String>,
    pub is_working_copy: bool,
}

/// Handle to a jj repository
pub struct RepoHandle {
    repo: Arc<ReadonlyRepo>,
    wc_commit_id: Option<CommitId>,
}

impl RepoHandle {
    /// Open a jj repository at the given path
    pub fn open(path: &Path) -> Result<Self> {
        // Load config using jj-cli's config system (same as gg)
        let settings = Self::load_settings(Some(path))?;

        let store_factories = StoreFactories::default();
        let working_copy_factories = default_working_copy_factories();

        let workspace = Workspace::load(&settings, path, &store_factories, &working_copy_factories)
            .context("Failed to load jj workspace")?;

        // Load the repo at head
        let repo = workspace
            .repo_loader()
            .load_at_head()
            .context("Failed to load repository")?;

        // Get the working copy commit ID from the repo view
        let workspace_name = workspace.workspace_name();
        let wc_commit_id = repo.view().get_wc_commit_id(workspace_name).cloned();

        Ok(Self { repo, wc_commit_id })
    }

    /// Load jj settings using jj-cli's config system
    fn load_settings(repo_path: Option<&Path>) -> Result<UserSettings> {
        let mut config_env = ConfigEnv::from_environment();

        // Get default config layers from jj-cli
        let default_layers = default_config_layers();

        // Add our own default layer for dojo-specific settings
        let dojo_layer =
            ConfigLayer::parse(ConfigSource::Default, include_str!("dojo_defaults.toml"))?;

        let mut layers = default_layers;
        layers.push(dojo_layer);

        // Build the raw config from environment
        let mut raw_config = config_from_environment(layers);

        // Load user config
        config_env.reload_user_config(&mut raw_config)?;

        // Load repo config if available
        if let Some(repo_path) = repo_path {
            config_env.reset_repo_path(repo_path);
            config_env.reload_repo_config(&mut raw_config)?;
        }

        // Resolve the config (handles scoped configs, etc.)
        let config = config_env.resolve_config(&raw_config)?;

        // Create UserSettings from the resolved config
        let settings = UserSettings::from_config(config)?;

        Ok(settings)
    }

    /// Get the log of commits
    pub fn log(&self, limit: usize) -> Result<Vec<CommitInfo>> {
        let mut commits = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Get all heads and walk backwards
        for head_id in self.repo.view().heads().iter() {
            self.collect_commits(head_id, &mut commits, &mut seen, limit)?;
            if commits.len() >= limit {
                break;
            }
        }

        // Sort by timestamp descending
        commits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        commits.truncate(limit);

        Ok(commits)
    }

    fn collect_commits(
        &self,
        commit_id: &CommitId,
        commits: &mut Vec<CommitInfo>,
        seen: &mut std::collections::HashSet<String>,
        limit: usize,
    ) -> Result<()> {
        if commits.len() >= limit {
            return Ok(());
        }

        let id_hex = commit_id.hex();
        if seen.contains(&id_hex) {
            return Ok(());
        }
        seen.insert(id_hex.clone());

        let store = self.repo.store();
        let commit = store.get_commit(commit_id)?;

        let info = self.commit_to_info(&commit)?;
        let parent_ids: Vec<CommitId> = commit.parent_ids().to_vec();
        commits.push(info);

        // Recurse into parents
        for parent_id in parent_ids {
            self.collect_commits(&parent_id, commits, seen, limit)?;
        }

        Ok(())
    }

    fn commit_to_info(&self, commit: &Commit) -> Result<CommitInfo> {
        let author_sig = commit.author();

        // Convert timestamp
        let millis = author_sig.timestamp.timestamp.0;
        let tz_offset_secs = author_sig.timestamp.tz_offset * 60;
        let offset = chrono::FixedOffset::east_opt(tz_offset_secs)
            .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).unwrap());
        let timestamp = chrono::DateTime::from_timestamp_millis(millis)
            .unwrap_or_default()
            .with_timezone(&offset);

        let is_wc = self
            .wc_commit_id
            .as_ref()
            .map(|wc| wc == commit.id())
            .unwrap_or(false);

        Ok(CommitInfo {
            commit_id: commit.id().hex(),
            change_id: commit.change_id().hex(),
            description: commit.description().to_string(),
            author: author_sig.name.clone(),
            timestamp,
            parent_ids: commit.parent_ids().iter().map(|id| id.hex()).collect(),
            is_working_copy: is_wc,
        })
    }
}
