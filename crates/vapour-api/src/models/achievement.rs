use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Achievement {
    pub apiname: String,
    pub achieved: u8,
    pub unlocktime: u64,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl Achievement {
    pub fn is_unlocked(&self) -> bool {
        self.achieved == 1
    }

    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.apiname)
    }
}
