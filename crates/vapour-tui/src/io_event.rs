#[derive(Debug)]
#[allow(dead_code)]
pub enum IoEvent {
    LoadLibrary,
    /// Fetch all friend IDs, then kick off the first page.
    LoadFriendIds,
    /// Fetch friend summaries starting at `page * 100`. Chains to next page on arrival.
    LoadFriendPage(usize),
    LoadWishlist,
    LoadNews,
    LoadGameDetail(u32),
    LoadAchievements(u32),
    RefreshAll,
    /// Look up game names for app IDs not in the user's library.
    LookupGameNames(Vec<u32>),
}
