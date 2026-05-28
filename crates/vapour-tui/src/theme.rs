use ratatui::style::Color;

#[allow(dead_code)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub highlight: Color,
    pub highlight_text: Color,
    pub tab_active: Color,
    pub tab_inactive: Color,
    pub online: Color,
    pub ingame: Color,
    pub offline: Color,
    pub border: Color,
    pub border_focused: Color,
    pub error: Color,
    pub muted: Color,
    pub status_bar_bg: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::Rgb(220, 220, 220),
            highlight: Color::Rgb(31, 111, 187),   // Steam blue
            highlight_text: Color::White,
            tab_active: Color::Rgb(31, 111, 187),
            tab_inactive: Color::Rgb(100, 100, 100),
            online: Color::Rgb(100, 214, 118),
            ingame: Color::Rgb(90, 160, 255),
            offline: Color::Rgb(100, 100, 100),
            border: Color::Rgb(60, 60, 60),
            border_focused: Color::Rgb(31, 111, 187),
            error: Color::Rgb(220, 80, 80),
            muted: Color::Rgb(140, 140, 140),
            status_bar_bg: Color::Rgb(25, 25, 35),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: Color::White,
            fg: Color::Rgb(30, 30, 30),
            highlight: Color::Rgb(31, 111, 187),
            highlight_text: Color::White,
            tab_active: Color::Rgb(31, 111, 187),
            tab_inactive: Color::Rgb(150, 150, 150),
            online: Color::Rgb(40, 160, 60),
            ingame: Color::Rgb(20, 100, 200),
            offline: Color::Rgb(150, 150, 150),
            border: Color::Rgb(200, 200, 200),
            border_focused: Color::Rgb(31, 111, 187),
            error: Color::Rgb(200, 40, 40),
            muted: Color::Rgb(120, 120, 120),
            status_bar_bg: Color::Rgb(230, 230, 240),
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            bg: Color::Rgb(40, 40, 40),
            fg: Color::Rgb(235, 219, 178),
            highlight: Color::Rgb(215, 153, 33),
            highlight_text: Color::Rgb(40, 40, 40),
            tab_active: Color::Rgb(215, 153, 33),
            tab_inactive: Color::Rgb(146, 131, 116),
            online: Color::Rgb(184, 187, 38),
            ingame: Color::Rgb(131, 165, 152),
            offline: Color::Rgb(146, 131, 116),
            border: Color::Rgb(80, 73, 69),
            border_focused: Color::Rgb(215, 153, 33),
            error: Color::Rgb(251, 73, 52),
            muted: Color::Rgb(146, 131, 116),
            status_bar_bg: Color::Rgb(50, 48, 47),
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            "gruvbox" => Self::gruvbox(),
            _ => Self::dark(),
        }
    }
}
