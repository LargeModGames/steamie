use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WishlistItem {
    pub appid: u32,
    pub name: String,
    pub priority: u32,
    pub added: u64,
    pub capsule: Option<String>,
}
