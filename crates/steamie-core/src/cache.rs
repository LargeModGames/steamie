use steamie_api::{Game, NewsItem, PlayerSummary, WishlistItem};

#[derive(Debug, Default)]
pub struct Cache {
    pub games: Vec<Game>,
    pub friends: Vec<PlayerSummary>,
    pub wishlist: Vec<WishlistItem>,
    pub news: Vec<NewsItem>,
}
