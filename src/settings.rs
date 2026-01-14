#[derive(Debug, Clone)]
pub struct Settings {
    pub diff: DiffSettings,
    pub commit_list: CommitListSettings,
    pub sidebar: SidebarSettings,
}

#[derive(Debug, Clone)]
pub struct DiffSettings {
    pub side_by_side_threshold: u32,
}

#[derive(Debug, Clone)]
pub struct CommitListSettings {
    pub min_width: u32,
    pub max_width: u32,
}

#[derive(Debug, Clone)]
pub struct SidebarSettings {
    pub min_width: u32,
    pub max_width: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            diff: DiffSettings {
                side_by_side_threshold: 800,
            },
            commit_list: CommitListSettings {
                min_width: 400,
                max_width: 800,
            },
            sidebar: SidebarSettings {
                min_width: 100,
                max_width: 180,
            },
        }
    }
}
