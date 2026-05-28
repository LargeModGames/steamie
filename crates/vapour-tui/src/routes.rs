use crate::io_event::IoEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteId {
    Library,
    GameDetail,
    Friends,
    Wishlist,
    News,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ActiveBlock {
    Library,
    GameDetail,
    Friends,
    Wishlist,
    News,
    Search,
    Help,
    Error,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub id: RouteId,
    pub active_block: ActiveBlock,
}

impl Route {
    pub fn library() -> Self {
        Self { id: RouteId::Library, active_block: ActiveBlock::Library }
    }

    pub fn game_detail() -> Self {
        Self { id: RouteId::GameDetail, active_block: ActiveBlock::GameDetail }
    }

    pub fn friends() -> Self {
        Self { id: RouteId::Friends, active_block: ActiveBlock::Friends }
    }

    pub fn wishlist() -> Self {
        Self { id: RouteId::Wishlist, active_block: ActiveBlock::Wishlist }
    }

    pub fn news() -> Self {
        Self { id: RouteId::News, active_block: ActiveBlock::News }
    }

    /// IoEvent to fire when this route first becomes active
    pub fn load_event(&self) -> Option<IoEvent> {
        match self.id {
            RouteId::Library => Some(IoEvent::LoadLibrary),
            RouteId::Friends => Some(IoEvent::LoadFriendIds),
            RouteId::Wishlist => Some(IoEvent::LoadWishlist),
            RouteId::News => Some(IoEvent::LoadNews),
            RouteId::GameDetail => None,
        }
    }
}
