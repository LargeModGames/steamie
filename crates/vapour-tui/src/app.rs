use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ratatui::widgets::ListState;
use tokio::sync::mpsc as tokio_mpsc;
use vapour_api::{Achievement, AppDetails, Game, NewsItem, PlayerSummary, WishlistItem};
use vapour_core::{ChatHistory, Config};
use vapour_protocol::{ChatMessage, Persona, PersonaState, RunCommand};

use crate::io_event::IoEvent;
use crate::protocol::{ProtocolCommand, ProtocolStatus};
use crate::routes::{ActiveBlock, Route, RouteId};

/// Minimum gap between outgoing typing pings while the user is composing.
const TYPING_THROTTLE: Duration = Duration::from_secs(4);

/// Steam-style library filter by app type. Cycles All → Games → Software/Tools with the `t` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTypeFilter {
    #[default]
    All,
    Games,
    SoftwareTools,
}

impl AppTypeFilter {
    pub fn cycle(self) -> Self {
        match self {
            Self::All => Self::Games,
            Self::Games => Self::SoftwareTools,
            Self::SoftwareTools => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Games => "Games",
            Self::SoftwareTools => "Software/Tools",
        }
    }

    /// Whether a game's appinfo type passes this filter. Untyped entries (`None`) count as games:
    /// the protocol path already drops DLC/music/video, and the Web-API fallback (which reports no
    /// type at all) only ever returns games — so `None` is game-ish by default and stays visible
    /// under `All` and `Games`. Only `Software/Tools` is strict, since it has no fallback meaning.
    fn matches(self, app_type: Option<&str>) -> bool {
        match self {
            Self::All => true,
            Self::Games => matches!(app_type, Some("game") | None),
            Self::SoftwareTools => matches!(app_type, Some("application" | "tool")),
        }
    }
}

/// In-memory state for one 1-on-1 conversation.
#[derive(Default)]
pub struct Conversation {
    /// Messages oldest-first, deduped on `(timestamp, ordinal)`.
    pub messages: Vec<ChatMessage>,
    pub unread: usize,
    /// When the partner's last typing notification lapses.
    pub peer_typing_until: Option<Instant>,
}

impl Conversation {
    pub fn is_typing(&self) -> bool {
        self.peer_typing_until.is_some_and(|until| Instant::now() < until)
    }
}

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
    pub app_type_filter: AppTypeFilter,
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
    // --- chat ---
    /// Per-partner conversation state, keyed by SteamID64.
    pub conversations: HashMap<u64, Conversation>,
    /// SteamID of the conversation currently open in the chat view.
    pub active_conversation: Option<u64>,
    /// Selection within the conversation list (indexes [`App::conversation_order`]).
    pub chat_list_state: ListState,
    /// Composer text for the active conversation.
    pub chat_input: String,
    /// Lines scrolled up from the bottom of the message history (0 = newest).
    pub chat_scroll_back: usize,
    /// When we last sent a typing ping (throttle).
    pub last_typing_sent: Option<Instant>,
    /// Disk-backed local history cache.
    pub chat_history: ChatHistory,
    /// Serialized off-thread sink for persisting a conversation snapshot to disk. Set once the
    /// protocol task is up; `None` until then. Keeps disk I/O off the render-critical App mutex.
    pub chat_persist_tx: Option<tokio_mpsc::UnboundedSender<(u64, Vec<ChatMessage>)>>,
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
        let mut chat_list_state = ListState::default();
        chat_list_state.select(Some(0));

        let chat_history = ChatHistory::new(config.chat.history_retention_days);

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
            app_type_filter: AppTypeFilter::default(),
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
            conversations: HashMap::new(),
            active_conversation: None,
            chat_list_state,
            chat_input: String::new(),
            chat_scroll_back: 0,
            last_typing_sent: None,
            chat_history,
            chat_persist_tx: None,
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
        let type_filter = self.app_type_filter;
        self.filtered_games = self
            .games
            .iter()
            .enumerate()
            .filter(|(_, g)| {
                let name_matches =
                    q.is_empty() || self.game_display_name(g).to_lowercase().contains(&q);
                name_matches && type_filter.matches(g.app_type.as_deref())
            })
            .map(|(i, _)| i)
            .collect();
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

    /// Whether a text-input mode (search or chat composer) currently owns the keyboard. Used by
    /// the event loop to route every key (including `q`) to the handler instead of quitting.
    pub fn is_text_input_active(&self) -> bool {
        self.is_searching || matches!(self.active_block(), ActiveBlock::ChatComposer)
    }

    /// Personas in display order: in-game first, then online, then offline; name within each group.
    /// Shared by the friends view and the "open chat" handler so their indices stay aligned.
    pub fn sorted_protocol_friends(&self) -> Vec<&Persona> {
        let mut friends: Vec<&Persona> = self.protocol_friends.iter().collect();
        friends.sort_by_key(|p| {
            let order = if p.game_app_id.is_some() {
                0u8
            } else if p.state != PersonaState::Offline && p.state != PersonaState::Invisible {
                1
            } else {
                2
            };
            (order, p.name.to_lowercase())
        });
        friends
    }

    /// Display name for a friend's SteamID, falling back to the numeric id.
    pub fn friend_name(&self, steamid: u64) -> String {
        self.protocol_friends
            .iter()
            .find(|p| p.steamid == steamid)
            .map(|p| p.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| steamid.to_string())
    }

    /// Conversation SteamIDs ordered most-recent-activity first.
    pub fn conversation_order(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self.conversations.keys().copied().collect();
        ids.sort_by_key(|id| {
            let last = self.conversations[id]
                .messages
                .last()
                .map(|m| m.timestamp)
                .unwrap_or(0);
            std::cmp::Reverse(last)
        });
        ids
    }

    pub fn total_unread(&self) -> usize {
        self.conversations.values().map(|c| c.unread).sum()
    }

    /// Whether the chat view is currently showing `steamid` (suppresses unread + notifications).
    pub fn is_viewing_conversation(&self, steamid: u64) -> bool {
        self.current_route().id == RouteId::Chat && self.active_conversation == Some(steamid)
    }

    /// Materialize a conversation, loading its on-disk history the first time it appears this
    /// session. This guarantees the in-memory message list already contains prior on-disk history
    /// *before* any save overwrites the file — without it, the first message for a not-yet-opened
    /// partner would truncate the saved history down to that single message.
    pub fn ensure_conversation(&mut self, steamid: u64) -> &mut Conversation {
        if !self.conversations.contains_key(&steamid) {
            let cached = self.chat_history.load(steamid);
            self.conversations.insert(
                steamid,
                Conversation {
                    messages: cached,
                    ..Default::default()
                },
            );
        }
        self.conversations
            .get_mut(&steamid)
            .expect("conversation was just ensured")
    }

    /// Open (or refocus) the conversation with `steamid`: load cached history, clear unread, focus
    /// the composer, and request a server history backfill.
    pub fn open_conversation(&mut self, steamid: u64) {
        self.active_conversation = Some(steamid);
        self.chat_scroll_back = 0;

        let convo = self.ensure_conversation(steamid);
        convo.unread = 0;

        self.navigation_stack = vec![Route {
            id: RouteId::Chat,
            active_block: ActiveBlock::ChatComposer,
        }];

        if let Some(tx) = &self.friend_cmd_tx {
            let _ = tx.send(RunCommand::GetRecentMessages { steamid });
        }
    }

    /// Send the composer's text to the active conversation. The message appears when Steam
    /// confirms it (via `MessageSent`), carrying the authoritative timestamp+ordinal.
    pub fn send_chat_message(&mut self) {
        let text = self.chat_input.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let Some(steamid) = self.active_conversation else {
            return;
        };
        if let Some(tx) = &self.friend_cmd_tx {
            let _ = tx.send(RunCommand::SendMessage {
                steamid,
                message: text,
            });
        }
        self.chat_input.clear();
        self.chat_scroll_back = 0;
    }

    /// Send a throttled typing ping for the active conversation.
    pub fn notify_typing(&mut self) {
        let Some(steamid) = self.active_conversation else {
            return;
        };
        let now = Instant::now();
        let due = self
            .last_typing_sent
            .is_none_or(|t| now.duration_since(t) >= TYPING_THROTTLE);
        if !due {
            return;
        }
        if let Some(tx) = &self.friend_cmd_tx {
            let _ = tx.send(RunCommand::SendTyping { steamid });
        }
        self.last_typing_sent = Some(now);
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
