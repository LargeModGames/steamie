use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerSummary {
    pub steamid: String,
    pub personaname: String,
    pub avatarfull: Option<String>,
    /// 0=offline, 1=online, 2=busy, 3=away, 4=snooze, 5=looking to trade, 6=looking to play
    pub personastate: u8,
    pub gameid: Option<String>,
    pub gameextrainfo: Option<String>,
}

impl PlayerSummary {
    pub fn persona_state_label(&self) -> &'static str {
        match self.personastate {
            1 => "Online",
            2 => "Busy",
            3 => "Away",
            4 => "Snooze",
            5 => "Looking to Trade",
            6 => "Looking to Play",
            _ => "Offline",
        }
    }

    pub fn is_in_game(&self) -> bool {
        self.gameid.is_some()
    }
}
