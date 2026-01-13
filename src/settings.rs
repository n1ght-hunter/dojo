#[derive(Debug, Clone)]
pub struct Settings {
    pub diff: DiffSettings,
}

#[derive(Debug, Clone)]
pub struct DiffSettings {
    pub side_by_side_threshold: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            diff: DiffSettings {
                side_by_side_threshold: 800,
            },
        }
    }
}
