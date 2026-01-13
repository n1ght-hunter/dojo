use anyhow::{Context, Result};
use futures::StreamExt;
use jj_cli::config::{ConfigEnv, config_from_environment, default_config_layers};
use jj_lib::backend::CommitId;
use jj_lib::commit::Commit;
use jj_lib::config::{ConfigLayer, ConfigSource};
use jj_lib::matchers::EverythingMatcher;
use jj_lib::merged_tree::TreeDiffEntry;
use jj_lib::object_id::ObjectId;
use jj_lib::repo::{ReadonlyRepo, Repo, StoreFactories};
use jj_lib::settings::UserSettings;
use jj_lib::store::Store;
use jj_lib::workspace::{Workspace, default_working_copy_factories};
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncReadExt;

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
    pub bookmarks: Vec<String>,
}

/// Kind of file change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
}

/// Information about a changed file
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub kind: ChangeKind,
    pub has_conflict: bool,
}

/// A line in a diff
#[derive(Debug, Clone)]
pub enum DiffLine {
    Context(String),
    Added(String),
    Removed(String),
    Hunk {
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
    },
}

/// A file's diff content
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub kind: ChangeKind,
    pub lines: Vec<DiffLine>,
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

    /// Get the changed files for a commit (compared to its first parent)
    pub async fn get_changed_files(&self, commit_id: &str) -> Result<Vec<FileChange>> {
        let commit_id = CommitId::try_from_hex(commit_id).context("Invalid commit ID hex")?;
        let store = self.repo.store();
        let commit = store.get_commit(&commit_id)?;

        // Get the tree for this commit
        let tree = commit.tree();

        // Get parent tree (use empty tree if no parents)
        let parent_tree = if let Some(parent_id) = commit.parent_ids().first() {
            let parent_commit = store.get_commit(parent_id)?;
            parent_commit.tree()
        } else {
            // Root commit - compare against empty tree
            self.repo.store().empty_merged_tree()
        };

        // Get the diff stream
        let mut tree_diff = parent_tree.diff_stream(&tree, &EverythingMatcher);

        // Collect changes
        let mut changes = Vec::new();

        while let Some(TreeDiffEntry { path, values }) = tree_diff.next().await {
            if let Ok(diff) = values {
                let before = &diff.before;
                let after = &diff.after;

                let kind = if before.is_present() && after.is_present() {
                    ChangeKind::Modified
                } else if before.is_absent() {
                    ChangeKind::Added
                } else {
                    ChangeKind::Deleted
                };

                let has_conflict = !after.is_resolved();

                changes.push(FileChange {
                    path: path.as_internal_file_string().to_string(),
                    kind,
                    has_conflict,
                });
            }
        }

        Ok(changes)
    }

    /// Get the diff for a specific file in a commit
    pub async fn get_file_diff(&self, commit_id: &str, file_path: &str) -> Result<FileDiff> {
        let commit_id = CommitId::try_from_hex(commit_id).context("Invalid commit ID hex")?;
        let store = self.repo.store();
        let commit = store.get_commit(&commit_id)?;

        let tree = commit.tree();

        // Get parent tree
        let parent_tree = if let Some(parent_id) = commit.parent_ids().first() {
            let parent_commit = store.get_commit(parent_id)?;
            parent_commit.tree()
        } else {
            self.repo.store().empty_merged_tree()
        };

        let repo_path = jj_lib::repo_path::RepoPath::from_internal_string(file_path)
            .context("Invalid file path")?;

        // Get old content
        let old_content = Self::get_file_content_from_tree(&parent_tree, repo_path, store).await;

        // Get new content
        let new_content = Self::get_file_content_from_tree(&tree, repo_path, store).await;

        // Determine change kind
        let kind = match (&old_content, &new_content) {
            (None, Some(_)) => ChangeKind::Added,
            (Some(_), None) => ChangeKind::Deleted,
            _ => ChangeKind::Modified,
        };

        // Compute diff
        let old_lines: Vec<&str> = old_content.as_deref().unwrap_or("").lines().collect();
        let new_lines: Vec<&str> = new_content.as_deref().unwrap_or("").lines().collect();

        let diff_lines = Self::compute_diff(&old_lines, &new_lines);

        Ok(FileDiff {
            path: file_path.to_string(),
            kind,
            lines: diff_lines,
        })
    }

    async fn get_file_content_from_tree(
        tree: &jj_lib::merged_tree::MergedTree,
        path: &jj_lib::repo_path::RepoPath,
        store: &Arc<Store>,
    ) -> Option<String> {
        let value = tree.path_value(path).ok()?;

        // Get the first resolved value
        let tree_value = value.as_resolved()?.as_ref()?;

        match tree_value {
            jj_lib::backend::TreeValue::File { id, .. } => {
                let mut reader = store.read_file(path, id).await.ok()?;
                let mut content = String::new();
                reader.read_to_string(&mut content).await.ok()?;
                Some(content)
            }
            _ => None,
        }
    }

    /// Simple line-by-line diff algorithm
    fn compute_diff(old_lines: &[&str], new_lines: &[&str]) -> Vec<DiffLine> {
        let mut result = Vec::new();

        // Use a simple LCS-based diff
        let lcs = Self::longest_common_subsequence(old_lines, new_lines);

        let mut old_idx = 0;
        let mut new_idx = 0;
        let mut lcs_idx = 0;

        while old_idx < old_lines.len() || new_idx < new_lines.len() {
            if lcs_idx < lcs.len() {
                let (lcs_old, lcs_new) = lcs[lcs_idx];

                // Output removed lines before the LCS match
                while old_idx < lcs_old {
                    result.push(DiffLine::Removed(old_lines[old_idx].to_string()));
                    old_idx += 1;
                }

                // Output added lines before the LCS match
                while new_idx < lcs_new {
                    result.push(DiffLine::Added(new_lines[new_idx].to_string()));
                    new_idx += 1;
                }

                // Output the context line (matching line)
                if old_idx < old_lines.len() && new_idx < new_lines.len() {
                    result.push(DiffLine::Context(old_lines[old_idx].to_string()));
                    old_idx += 1;
                    new_idx += 1;
                }

                lcs_idx += 1;
            } else {
                // No more LCS matches, output remaining lines
                while old_idx < old_lines.len() {
                    result.push(DiffLine::Removed(old_lines[old_idx].to_string()));
                    old_idx += 1;
                }
                while new_idx < new_lines.len() {
                    result.push(DiffLine::Added(new_lines[new_idx].to_string()));
                    new_idx += 1;
                }
            }
        }

        result
    }

    /// Compute longest common subsequence, returns indices of matching lines
    fn longest_common_subsequence(old: &[&str], new: &[&str]) -> Vec<(usize, usize)> {
        let m = old.len();
        let n = new.len();

        if m == 0 || n == 0 {
            return Vec::new();
        }

        // Build LCS length table
        let mut dp = vec![vec![0usize; n + 1]; m + 1];

        for i in 1..=m {
            for j in 1..=n {
                if old[i - 1] == new[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        // Backtrack to find the actual LCS
        let mut result = Vec::new();
        let mut i = m;
        let mut j = n;

        while i > 0 && j > 0 {
            if old[i - 1] == new[j - 1] {
                result.push((i - 1, j - 1));
                i -= 1;
                j -= 1;
            } else if dp[i - 1][j] > dp[i][j - 1] {
                i -= 1;
            } else {
                j -= 1;
            }
        }

        result.reverse();
        result
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

        // Get bookmarks pointing to this commit
        let bookmarks: Vec<String> = self
            .repo
            .view()
            .bookmarks()
            .filter_map(|(name, target)| {
                if target.local_target.added_ids().any(|id| id == commit.id()) {
                    Some(name.as_str().to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(CommitInfo {
            commit_id: commit.id().hex(),
            change_id: commit.change_id().hex(),
            description: commit.description().to_string(),
            author: author_sig.name.clone(),
            timestamp,
            parent_ids: commit.parent_ids().iter().map(|id| id.hex()).collect(),
            is_working_copy: is_wc,
            bookmarks,
        })
    }
}

impl std::fmt::Debug for RepoHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepoHandle")
            .field("wc_commit_id", &self.wc_commit_id)
            .finish_non_exhaustive()
    }
}
