use qrcode::{QrCode, render::unicode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    app::App,
    protocol::{ProtocolGuardKind, ProtocolStatus},
    theme::Theme,
};

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let modal = centered_rect(area, 72, 80);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Steam Login ");
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    match &app.protocol_status {
        ProtocolStatus::Connecting => {
            draw_message(f, inner, theme, "Connecting to Steam…", Some("Esc cancel"))
        }
        ProtocolStatus::AwaitingQrScan { qr_url } => draw_qr(f, inner, theme, qr_url),
        ProtocolStatus::AwaitingGuardCode { kind } => {
            draw_guard(f, inner, theme, kind, &app.protocol_input)
        }
        _ => {}
    }
}

fn draw_message(f: &mut Frame, area: Rect, theme: &Theme, message: &str, footer: Option<&str>) {
    let mut text = Text::from(vec![Line::from(message)]);
    if let Some(footer) = footer {
        text.extend(Text::from(vec![Line::from(""), Line::from(footer)]));
    }
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn draw_qr(f: &mut Frame, area: Rect, theme: &Theme, qr_url: &str) {
    let qr = render_qr(qr_url).unwrap_or_else(|| vec!["Unable to render QR".to_owned()]);
    let mut lines: Vec<Line> = qr.iter().map(|line| Line::from(line.as_str())).collect();
    lines.push(Line::from(""));
    lines.push(Line::from("Scan with the Steam mobile app"));
    lines.push(Line::from(qr_url));
    lines.push(Line::from(""));
    lines.push(Line::from("Esc cancel"));

    let paragraph = Paragraph::new(Text::from(lines))
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_guard(f: &mut Frame, area: Rect, theme: &Theme, kind: &ProtocolGuardKind, input: &str) {
    let prompt = match kind {
        ProtocolGuardKind::EmailCode => "Enter the Steam Guard email code",
        ProtocolGuardKind::DeviceCode => "Enter the Steam Guard code from your authenticator",
        ProtocolGuardKind::DeviceConfirmation => "Approve the login in the Steam mobile app",
    };

    let mut lines = vec![Line::from(prompt), Line::from("")];
    match kind {
        ProtocolGuardKind::EmailCode | ProtocolGuardKind::DeviceCode => {
            lines.push(Line::from(format!("Code: {}_", input)));
            lines.push(Line::from(""));
            lines.push(Line::from("Enter submit  Esc cancel"));
        }
        ProtocolGuardKind::DeviceConfirmation => {
            lines.push(Line::from("Waiting for confirmation…"));
            lines.push(Line::from(""));
            lines.push(Line::from("Esc cancel"));
        }
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.fg).add_modifier(Modifier::BOLD))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn render_qr(value: &str) -> Option<Vec<String>> {
    let qr = QrCode::new(value.as_bytes()).ok()?;
    let rendered = qr
        .render::<unicode::Dense1x2>()
        .quiet_zone(false)
        .dark_color(unicode::Dense1x2::Dark)
        .light_color(unicode::Dense1x2::Light)
        .build();
    Some(rendered.lines().map(str::to_owned).collect())
}

fn centered_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}
