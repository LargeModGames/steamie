use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct NewsItem {
    pub gid: String,
    pub title: String,
    pub url: String,
    pub feedlabel: String,
    pub date: u64,
    pub appid: u32,
    pub contents: Option<String>,
}
