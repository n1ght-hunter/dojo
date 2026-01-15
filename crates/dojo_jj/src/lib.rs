mod error;
mod repo;
pub mod worker;

pub use error::{DiffsError, FilesError, RefreshError, StatsError, UpdateDescriptionError};
pub use repo::{
    ChangeKind, CommitInfo, DiffLine, DiffSegment, FileChange, FileDiff, FileStats, ParentEdge,
    RepoHandle,
};
pub use worker::{WorkspaceCommand, WorkspaceEvent};
