//! The colours the desktop paints with, for the light and the dark appearance.
//!
//! The compositor blurs and saturates the wallpaper behind every glass shape;
//! these are what the desktop lays over that.

use crownui::prelude::{Color, Material, MaterialBlur, MaterialShadow, Vector, rgb, rgba};

/// One kind of glass: the tint over the blur and the lit rim around it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glass {
    pub tint: Color,
    pub rim: Color,
}

/// The light a widget's glass catches: a hairline all around its edge, a glow
/// just inside it, and a streak where the light strikes its corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sheen {
    pub rim: Color,
    pub glow: Color,
    pub specular: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dial {
    /// The face's light, from the top-left to the bottom-right.
    pub face: [Color; 3],
    pub numerals: Color,
    pub hour_tick: Color,
    pub minute_tick: Color,
    pub hands: Color,
    pub seconds: Color,
    pub caption_fill: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub accent: Color,
    pub ink: Color,
    pub ink_secondary: Color,
    /// The tint over the blur behind the widgets.
    pub frost: Color,
    pub sheen: Sheen,
    /// Menus and the widget gallery: thicker glass that reads over anything.
    pub panel: Glass,
    pub separator: Color,
    /// Text on the panel glass.
    pub panel_ink: Color,
    pub dial: Dial,
}

/// The blur behind every glass shape on the desktop: the compositor's, or
/// crownui's own where the compositor has none. A surface has one material,
/// so every shape shares it and tints its own.
pub const MATERIAL: Material = Material {
    corner_radius: 0.0,
    border_width: 0.0,
    blur: Some(MaterialBlur {
        radius: 40.0,
        tint: Color::new(0.0, 0.0, 0.0, 0.0),
        saturation: 1.8,
    }),
    shadow: Some(MaterialShadow {
        radius: 18.0,
        offset: Vector::new(0.0, 6.0),
        color: Color::new(0.0, 0.09, 0.25, 0.10),
    }),
};

const LIGHT: Palette = Palette {
    dark: false,
    accent: rgb(0x007AFF),
    ink: rgb(0x000206),
    ink_secondary: rgb(0x1F3E50),
    frost: rgba(0xFFFFFF61),
    sheen: Sheen {
        rim: rgba(0xFFFFFF99),
        glow: rgba(0xFFFFFF80),
        specular: rgba(0xFFFFFFF7),
    },
    panel: Glass {
        tint: rgba(0xFAF6F24D),
        rim: rgba(0xFFFFFFA6),
    },
    separator: rgba(0x0000001F),
    panel_ink: rgba(0x1E1B18E6),
    dial: Dial {
        face: [rgba(0xFFFFFFB8), rgba(0xFFFFFF85), rgba(0xFFFFFF5C)],
        numerals: rgb(0x0A0A0A),
        hour_tick: rgba(0x1A1A1AC0),
        minute_tick: rgba(0x3A3A3A80),
        hands: rgb(0x080808),
        seconds: rgb(0x6E6E6E),
        caption_fill: rgba(0x30303014),
    },
};

const DARK: Palette = Palette {
    dark: true,
    accent: rgb(0x0A84FF),
    ink: rgb(0xFFFFFF),
    ink_secondary: rgba(0xD8E6F0D0),
    frost: rgba(0x1C1C2073),
    sheen: Sheen {
        rim: rgba(0xFFFFFF2E),
        glow: rgba(0xFFFFFF1A),
        specular: rgba(0xFFFFFFB3),
    },
    panel: Glass {
        tint: rgba(0x26262C66),
        rim: rgba(0xFFFFFF2E),
    },
    separator: rgba(0xFFFFFF24),
    panel_ink: rgba(0xFFFFFFEB),
    dial: Dial {
        face: [rgba(0x4A4A4E8C), rgba(0x2A2A2E80), rgba(0x1A1A1E73)],
        numerals: rgb(0xF4F4F4),
        hour_tick: rgba(0xF4F4F4C0),
        minute_tick: rgba(0xC8C8C870),
        hands: rgb(0xF6F6F6),
        seconds: rgb(0xA0A0A0),
        caption_fill: rgba(0xFFFFFF18),
    },
};

impl Palette {
    pub fn new(dark: bool, accent: Color) -> Self {
        Self {
            accent,
            ..if dark { DARK } else { LIGHT }
        }
    }

    /// A highlighted menu row: the panel's glass, lit brighter.
    pub fn highlight(&self) -> Color {
        if self.dark {
            rgba(0xFFFFFF2E)
        } else {
            rgba(0xFFFFFFE6)
        }
    }
}
