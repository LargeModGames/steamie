//! Disk-backed local cache of 1-on-1 chat history.
//!
//! One JSON file per conversation partner under `~/.local/state/vapour/chat/<steamid>.json`
//! (same state-dir resolver as [`crate::auth`]). Reads are best-effort: a missing or corrupt
//! file yields an empty history rather than an error. Writes prune to the configured retention
//! window first.

use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use vapour_protocol::ChatMessage;

const SECONDS_PER_DAY: u64 = 86_400;

/// Local chat-history store. Cheap to clone (just a path + retention setting).
#[derive(Debug, Clone)]
pub struct ChatHistory {
    dir: PathBuf,
    retention_days: u32,
}

impl ChatHistory {
    /// Store rooted at the default state directory.
    pub fn new(retention_days: u32) -> Self {
        Self {
            dir: chat_history_dir(),
            retention_days,
        }
    }

    /// Store rooted at an explicit directory (used by tests).
    pub fn with_dir(dir: PathBuf, retention_days: u32) -> Self {
        Self {
            dir,
            retention_days,
        }
    }

    /// Load a conversation's cached messages, oldest first. Missing or unreadable files yield an
    /// empty history — the local cache is best-effort and never fatal.
    pub fn load(&self, steamid: u64) -> Vec<ChatMessage> {
        match fs::read_to_string(self.path(steamid)) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// Persist a conversation, pruning to the retention window first.
    pub fn save(&self, steamid: u64, messages: &[ChatMessage]) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("could not create chat history dir {}", self.dir.display()))?;
        let kept = prune(messages, self.retention_days, now_unix());
        let path = self.path(steamid);
        let raw = serde_json::to_string(&kept).context("could not serialize chat history")?;
        let mut file = fs::File::create(&path)
            .with_context(|| format!("could not open chat history {}", path.display()))?;
        file.write_all(raw.as_bytes())
            .with_context(|| format!("could not write chat history {}", path.display()))?;
        file.sync_all().ok();
        Ok(())
    }

    fn path(&self, steamid: u64) -> PathBuf {
        self.dir.join(format!("{steamid}.json"))
    }
}

/// Merge `incoming` messages into `existing`, deduping on the stable `(timestamp, ordinal)` key
/// and keeping the result sorted oldest-first. Returns `true` if anything new was added.
pub fn merge(
    existing: &mut Vec<ChatMessage>,
    incoming: impl IntoIterator<Item = ChatMessage>,
) -> bool {
    let mut added = false;
    for msg in incoming {
        let duplicate = existing
            .iter()
            .any(|m| m.timestamp == msg.timestamp && m.ordinal == msg.ordinal);
        if !duplicate {
            existing.push(msg);
            added = true;
        }
    }
    if added {
        existing.sort_by(|a, b| {
            a.timestamp
                .cmp(&b.timestamp)
                .then(a.ordinal.cmp(&b.ordinal))
        });
    }
    added
}

/// Drop messages older than the retention window. `retention_days == 0` keeps everything.
fn prune(messages: &[ChatMessage], retention_days: u32, now_unix: u64) -> Vec<ChatMessage> {
    if retention_days == 0 {
        return messages.to_vec();
    }
    let cutoff = now_unix.saturating_sub(retention_days as u64 * SECONDS_PER_DAY);
    messages
        .iter()
        .filter(|m| m.timestamp as u64 >= cutoff)
        .cloned()
        .collect()
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn chat_history_dir() -> PathBuf {
    dirs::state_dir()
        .or_else(dirs::config_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("vapour")
        .join("chat")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn msg(timestamp: u32, ordinal: u32, from_local: bool, text: &str) -> ChatMessage {
        ChatMessage {
            steamid: 76561198000000002,
            message: text.to_owned(),
            timestamp,
            ordinal,
            from_local,
        }
    }

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "vapour-chat-tests-{}-{}",
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn save_then_load_round_trips() -> Result<()> {
        let dir = unique_test_dir();
        // retention 0 = keep all, so the small test timestamps survive the prune-on-save.
        let store = ChatHistory::with_dir(dir.clone(), 0);
        let messages = vec![msg(100, 0, false, "hi"), msg(200, 0, true, "hey")];

        store.save(42, &messages)?;
        assert_eq!(store.load(42), messages);

        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn load_missing_conversation_is_empty() {
        let store = ChatHistory::with_dir(unique_test_dir(), 30);
        assert!(store.load(999).is_empty());
    }

    #[test]
    fn merge_dedupes_by_timestamp_ordinal_and_sorts() {
        let mut existing = vec![msg(200, 0, true, "second")];
        let added = merge(
            &mut existing,
            vec![
                msg(100, 0, false, "first"),
                msg(200, 0, true, "second"), // duplicate (same ts+ordinal) — dropped
                msg(200, 1, false, "also-200"),
            ],
        );
        assert!(added);
        assert_eq!(existing.len(), 3);
        assert_eq!(existing[0].message, "first");
        assert_eq!(existing[1].message, "second");
        assert_eq!(existing[2].message, "also-200");

        // Re-merging the same set adds nothing.
        assert!(!merge(&mut existing, vec![msg(100, 0, false, "first")]));
    }

    #[test]
    fn prune_drops_old_and_respects_keep_all() {
        let now = 10 * SECONDS_PER_DAY; // 10 days since epoch
        let messages = vec![
            msg((2 * SECONDS_PER_DAY) as u32, 0, false, "old"), // 8 days ago
            msg((9 * SECONDS_PER_DAY) as u32, 0, true, "recent"), // 1 day ago
        ];

        let kept = prune(&messages, 3, now);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].message, "recent");

        assert_eq!(prune(&messages, 0, now).len(), 2);
    }
}
