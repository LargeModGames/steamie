use std::collections::HashMap;
use std::sync::mpsc;

use ratatui::widgets::ListState;
use tokio::sync::mpsc as tokio_mpsc;
use vapour_api::{Achievement, AppDetails, Game, NewsItem, PlayerSummary, WishlistItem};
use vapour_core::Config;
use vapour_protocol::{Persona, PersonaState, RunCommand};

use crate::io_event::IoEvent;
use crate::protocol::{ProtocolCommand, ProtocolStatus};
use crate::routes::{ActiveBlock, Route};

pub struct App {
    pub navigation_stack: Vec<Route>,
    pub games: Vec<Game>,
    pub filtered_games: Vec<usize>,
    pub friend_ids: Vec<String>,     // all IDs from API
    pub friends: Vec<PlayerSummary>, // summaries loaded so far
    pub wishlist: Vec<WishlistItem>,
    pub recently_played_appids: Vec<u32>,
    pub news_feed: Vec<NewsItem>,
    pub selected_game: Option<AppDetails>,
    pub achievements: Vec<Achievement>,
    pub library_state: ListState,
    pub friends_state: ListState,
    pub wishlist_state: ListState,
    pub news_state: ListState,
    pub achievements_state: ListState,
    pub search_input: String,
    pub is_searching: bool,
    pub pending_g: bool,
    pub loading: ViewLoading,
    pub error: Option<String>,
    pub io_tx: mpsc::Sender<IoEvent>,
    pub protocol_status: ProtocolStatus,
    pub protocol_input: String,
    pub protocol_tx: tokio_mpsc::UnboundedSender<ProtocolCommand>,
    pub protocol_friends: Vec<Persona>,
    pub friend_cmd_tx: Option<tokio_mpsc::UnboundedSender<RunCommand>>,
    /// Names for app IDs not in the user's own library, fetched from the Store API.
    pub game_name_cache: HashMap<u32, String>,
    pub own_persona_state: PersonaState,
    #[allow(dead_code)]
    pub config: Config,
}

/// Per-view loading flags — lets the UI stay interactive while any single view loads.
#[derive(Default)]
pub struct ViewLoading {
    pub library: bool,
    pub friends: bool,
    pub wishlist: bool,
    pub news: bool,
    pub game_detail: bool,
}

impl App {
    pub fn new(
        io_tx: mpsc::Sender<IoEvent>,
        protocol_tx: tokio_mpsc::UnboundedSender<ProtocolCommand>,
        config: Config,
    ) -> Self {
        let mut library_state = ListState::default();
        library_state.select(Some(0));
        let mut friends_state = ListState::default();
        friends_state.select(Some(0));
        let mut wishlist_state = ListState::default();
        wishlist_state.select(Some(0));
        let mut news_state = ListState::default();
        news_state.select(Some(0));
        let mut achievements_state = ListState::default();
        achievements_state.select(Some(0));

        Self {
            navigation_stack: vec![Route::library()],
            games: vec![],
            filtered_games: vec![],
            friend_ids: vec![],
            friends: vec![],
            wishlist: vec![],
            recently_played_appids: vec![],
            news_feed: vec![],
            selected_game: None,
            achievements: vec![],
            library_state,
            friends_state,
            wishlist_state,
            news_state,
            achievements_state,
            search_input: String::new(),
            is_searching: false,
            pending_g: false,
            loading: ViewLoading::default(),
            error: None,
            io_tx,
            protocol_status: ProtocolStatus::Disconnected,
            protocol_input: String::new(),
            protocol_tx,
            protocol_friends: vec![],
            friend_cmd_tx: None,
            game_name_cache: HashMap::new(),
            own_persona_state: PersonaState::Online,
            config,
        }
    }

    pub fn current_route(&self) -> &Route {
        self.navigation_stack
            .last()
            .expect("navigation stack is never empty")
    }

    pub fn push_route(&mut self, route: Route) {
        self.navigation_stack.push(route);
    }

    pub fn pop_route(&mut self) {
        if self.navigation_stack.len() > 1 {
            self.navigation_stack.pop();
        }
    }

    pub fn active_block(&self) -> &ActiveBlock {
        &self.current_route().active_block
    }

    pub fn dispatch(&self, event: IoEvent) {
        let _ = self.io_tx.send(event);
    }

    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
        self.loading = ViewLoading::default();
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn protocol_modal_active(&self) -> bool {
        self.protocol_status.modal_visible()
    }

    pub fn submit_guard_code(&mut self) {
        if self.protocol_input.is_empty() {
            return;
        }

        let code = std::mem::take(&mut self.protocol_input);
        let _ = self
            .protocol_tx
            .send(ProtocolCommand::SubmitGuardCode(code));
        self.protocol_status = ProtocolStatus::Connecting;
    }

    pub fn update_search(&mut self) {
        let q = self.search_input.to_lowercase();
        let filtered_games = if q.is_empty() {
            (0..self.games.len()).collect()
        } else {
            self.games
                .iter()
                .enumerate()
                .filter(|(_, g)| self.game_display_name(g).to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect()
        };
        self.filtered_games = filtered_games;
        self.library_state
            .select(if self.filtered_games.is_empty() {
                None
            } else {
                Some(0)
            });
    }

    pub fn visible_games(&self) -> Vec<&Game> {
        self.filtered_games
            .iter()
            .map(|&i| &self.games[i])
            .collect()
    }

    pub fn game_display_name<'a>(&'a self, game: &'a Game) -> &'a str {
        game.name
            .as_deref()
            .filter(|name| !name.is_empty())
            .or_else(|| {
                self.game_name_cache
                    .get(&game.appid)
                    .map(String::as_str)
                    .filter(|name| !name.is_empty())
            })
            .unwrap_or("Unknown Game")
    }

    pub fn scroll_down(state: &mut ListState, len: usize) {
        if len == 0 {
            return;
        }
        let next = state.selected().map_or(0, |i| (i + 1).min(len - 1));
        state.select(Some(next));
    }

    pub fn scroll_up(state: &mut ListState) {
        let prev = state.selected().map_or(0, |i| i.saturating_sub(1));
        state.select(Some(prev));
    }

    pub fn scroll_top(state: &mut ListState) {
        state.select(Some(0));
    }

    pub fn scroll_bottom(state: &mut ListState, len: usize) {
        if len > 0 {
            state.select(Some(len - 1));
        }
    }
}
