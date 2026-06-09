use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use vapour_protocol::{ChatMessage, PersonaState};

use crate::app::App;
use crate::routes::ActiveBlock;
use crate::theme::Theme;

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    draw_conversation_list(f, app, theme, chunks[0]);
    draw_active_conversation(f, app, theme, chunks[1]);
}

fn draw_conversation_list(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let order = app.conversation_order();
    let focused = matches!(app.active_block(), ActiveBlock::Chat);
    let border_color = if focused {
        theme.border_focused
    } else {
        theme.border
    };
    let title = format!(" Chats ({}) ", order.len());

    if order.is_empty() {
        let hint = Paragraph::new(
            "No conversations yet.\n\nOpen one from the Friends tab\n(press Enter on a friend).",
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.muted))
        .block(list_block(title, border_color))
        .wrap(Wrap { trim: true });
        f.render_widget(hint, area);
        return;
    }

    let items: Vec<ListItem> = order
        .iter()
        .map(|&id| {
            let convo = &app.conversations[&id];
            let persona = app.protocol_friends.iter().find(|p| p.steamid == id);
            let in_game = persona.and_then(|p| p.game_app_id).is_some();
            let (dot, color) = match persona.map(|p| &p.state) {
                None | Some(PersonaState::Offline) | Some(PersonaState::Invisible) => {
                    ("○", theme.offline)
                }
                Some(_) if in_game => ("●", theme.ingame),
                Some(_) => ("●", theme.online),
            };

            let mut spans = vec![
                Span::styled(format!("{dot} "), Style::default().fg(color)),
                Span::styled(app.friend_name(id), Style::default().fg(theme.fg)),
            ];
            if convo.is_typing() {
                spans.push(Span::styled(
                    "  typing…",
                    Style::default()
                        .fg(theme.muted)
                        .add_modifier(Modifier::ITALIC),
                ));
            } else if convo.unread > 0 {
                spans.push(Span::styled(
                    format!("  ({})", convo.unread),
                    Style::default()
                        .fg(theme.ingame)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(list_block(title, border_color))
        .highlight_style(
            Style::default()
                .bg(theme.highlight)
                .fg(theme.highlight_text)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = clamped_list_state(&app.chat_list_state, order.len());
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_active_conversation(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let Some(steamid) = app.active_conversation else {
        let hint = Paragraph::new(
            "Select a conversation and press Enter,\nor open one from the Friends tab.",
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.muted))
        .block(list_block(" Conversation ".to_owned(), theme.border))
        .wrap(Wrap { trim: true });
        f.render_widget(hint, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let partner = app.friend_name(steamid);
    let convo = app.conversations.get(&steamid);

    // --- message history ---
    let inner_width = chunks[0].width.saturating_sub(2) as usize;
    let inner_height = chunks[0].height.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = Vec::new();
    if let Some(convo) = convo {
        for msg in &convo.messages {
            lines.extend(render_message_lines(msg, &partner, theme, inner_width));
        }
        if convo.is_typing() {
            lines.push(Line::from(Span::styled(
                format!("{partner} is typing…"),
                Style::default()
                    .fg(theme.muted)
                    .add_modifier(Modifier::ITALIC),
            )));
        }
    }

    // Anchor to the newest message, offset upward by the user's scroll-back.
    let total = lines.len();
    let max_scroll = total.saturating_sub(inner_height);
    let scroll_back = app.chat_scroll_back.min(max_scroll);
    let scroll_y = (max_scroll - scroll_back) as u16;

    let presence = app
        .protocol_friends
        .iter()
        .find(|p| p.steamid == steamid)
        .map(|p| !matches!(p.state, PersonaState::Offline | PersonaState::Invisible))
        .map(|online| if online { "online" } else { "offline" })
        .unwrap_or("offline");
    let title = format!(" {partner} — {presence} ");

    let history = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(title),
        )
        .scroll((scroll_y, 0));
    f.render_widget(history, chunks[0]);

    // --- composer ---
    let composing = matches!(app.active_block(), ActiveBlock::ChatComposer);
    let border_color = if composing {
        theme.border_focused
    } else {
        theme.border
    };
    let cursor = if composing { "_" } else { "" };
    // The composer is a single text row, so show only the tail of the input that fits, keeping the
    // caret visible instead of letting it scroll off into clipped wrapped rows.
    let inner_w = chunks[1].width.saturating_sub(2) as usize;
    let avail = inner_w.saturating_sub(3); // "> " prefix + trailing cursor
    let count = app.chat_input.chars().count();
    let shown: String = if count > avail {
        app.chat_input.chars().skip(count - avail).collect()
    } else {
        app.chat_input.clone()
    };
    let composer = Paragraph::new(format!("> {shown}{cursor}")).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(" Message "),
    );
    f.render_widget(composer, chunks[1]);
}

fn list_block(title: String, border_color: ratatui::style::Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title)
}

fn clamped_list_state(src: &ListState, len: usize) -> ListState {
    let mut state = *src;
    if len == 0 {
        state.select(None);
    } else {
        let sel = src.selected().unwrap_or(0).min(len - 1);
        state.select(Some(sel));
    }
    state
}

/// Render one message as wrapped lines: a bold author label on the first line, continuation
/// lines indented to align under the body.
fn render_message_lines(
    msg: &ChatMessage,
    partner: &str,
    theme: &Theme,
    width: usize,
) -> Vec<Line<'static>> {
    let (raw_label, label_color) = if msg.from_local {
        ("You".to_owned(), theme.online)
    } else {
        (partner.to_owned(), theme.ingame)
    };
    // Cap the author label so a very long persona name can't consume the whole line width.
    let label = truncate_label(&raw_label, (width / 3).clamp(8, 24));
    let indent = label.chars().count() + 2; // "label: "
    let body_width = width.saturating_sub(indent).max(8);
    let wrapped = wrap_text(&msg.message, body_width);

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, chunk) in wrapped.iter().enumerate() {
        if i == 0 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{label}: "),
                    Style::default()
                        .fg(label_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(chunk.clone(), Style::default().fg(theme.fg)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw(" ".repeat(indent)),
                Span::styled(chunk.clone(), Style::default().fg(theme.fg)),
            ]));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("{label}: "),
            Style::default()
                .fg(label_color)
                .add_modifier(Modifier::BOLD),
        )));
    }
    lines
}

/// Truncate `name` to at most `max` characters, appending `…` when shortened.
fn truncate_label(name: &str, max: usize) -> String {
    if name.chars().count() <= max {
        return name.to_owned();
    }
    let mut out: String = name.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Greedy word-wrap to `width` columns, hard-splitting words longer than the width.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for raw in text.split('\n') {
        let mut current = String::new();
        let mut current_len = 0usize;
        for word in raw.split_whitespace() {
            let word_len = word.chars().count();
            if current_len == 0 {
                push_word(
                    &mut lines,
                    &mut current,
                    &mut current_len,
                    word,
                    word_len,
                    width,
                );
            } else if current_len + 1 + word_len <= width {
                current.push(' ');
                current.push_str(word);
                current_len += 1 + word_len;
            } else {
                lines.push(std::mem::take(&mut current));
                current_len = 0;
                push_word(
                    &mut lines,
                    &mut current,
                    &mut current_len,
                    word,
                    word_len,
                    width,
                );
            }
        }
        lines.push(current);
    }
    lines
}

/// Place `word` into `current`, hard-splitting and flushing full-width chunks if it is too long.
fn push_word(
    lines: &mut Vec<String>,
    current: &mut String,
    current_len: &mut usize,
    word: &str,
    word_len: usize,
    width: usize,
) {
    if word_len <= width {
        *current = word.to_owned();
        *current_len = word_len;
        return;
    }
    let mut chars: Vec<char> = word.chars().collect();
    while chars.len() > width {
        let chunk: String = chars[..width].iter().collect();
        lines.push(chunk);
        chars.drain(..width);
    }
    *current = chars.into_iter().collect();
    *current_len = current.chars().count();
}

#[cfg(test)]
mod tests {
    use super::wrap_text;

    #[test]
    fn wraps_words_to_width() {
        let lines = wrap_text("the quick brown fox", 9);
        assert_eq!(lines, vec!["the quick", "brown fox"]);
    }

    #[test]
    fn hard_splits_overlong_word() {
        let lines = wrap_text("abcdefghij", 4);
        assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn preserves_explicit_newlines() {
        let lines = wrap_text("a\nb", 10);
        assert_eq!(lines, vec!["a", "b"]);
    }
}
