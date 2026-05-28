use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Game {
    pub appid: u32,
    pub name: Option<String>,
    pub playtime_forever: u32,
    pub img_icon_url: Option<String>,
    pub rtime_last_played: Option<u64>,
}

impl Game {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("Unknown Game")
    }

    pub fn playtime_hours(&self) -> f32 {
        self.playtime_forever as f32 / 60.0
    }
}
