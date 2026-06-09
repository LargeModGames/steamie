use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppDetails {
    pub steam_appid: u32,
    pub name: String,
    pub short_description: Option<String>,
    pub header_image: Option<String>,
    pub categories: Option<Vec<Category>>,
    pub genres: Option<Vec<Genre>>,
    pub release_date: Option<ReleaseDate>,
    pub metacritic: Option<Metacritic>,
    pub developers: Option<Vec<String>>,
    pub publishers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Category {
    pub id: u32,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Genre {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseDate {
    pub coming_soon: bool,
    pub date: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Metacritic {
    pub score: u8,
    pub url: String,
}
