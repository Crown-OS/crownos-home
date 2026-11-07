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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dial {
    /// The face's light, from the top-right to the bottom-left.
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
    pub frost: Glass,
    /// Menus and the widget gallery: thicker glass that reads over anything.
    pub panel: Glass,
    pub separator: Color,
    /// Text on the panel glass.
    pub panel_ink: Color,
    /// Light caught on the rim of the widgets' glass.
    pub specular: Color,
    pub dial: Dial,
}

/// How much more opaque glass gets when the compositor cannot blur: an
/// unblurred wallpaper behind text is unreadable.
const SOLID_BOOST: f32 = 0.35;

/// The blur the compositor runs behind every glass shape on the desktop. A
/// surface has one material, so every shape shares it and tints its own.
pub const MATERIAL: Material = Material {
    corner_radius: 0.0,
    border_width: 0.0,
    blur: Some(MaterialBlur {
        radius: 30.0,
        tint: Color::new(0.0, 0.0, 0.0, 0.0),
        saturation: 1.5,
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
    frost: Glass {
        tint: rgba(0xFFFFFF73),
        rim: rgba(0xECFCFF9C),
    },
    panel: Glass {
        tint: rgba(0xFAF6F24D),
        rim: rgba(0xFFFFFFA6),
    },
    separator: rgba(0x0000001F),
    panel_ink: rgba(0x1E1B18E6),
    specular: rgba(0xFFFFFFF2),
    dial: Dial {
        face: [rgba(0xECECECA6), rgba(0xFFFFFFC8), rgba(0xE2E2E2C0)],
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
    frost: Glass {
        tint: rgba(0x0A121C6A),
        rim: rgba(0xFFFFFF40),
    },
    panel: Glass {
        tint: rgba(0x26262C66),
        rim: rgba(0xFFFFFF2E),
    },
    separator: rgba(0xFFFFFF24),
    panel_ink: rgba(0xFFFFFFEB),
    specular: rgba(0xFFFFFFE0),
    dial: Dial {
        face: [rgba(0x262626A0), rgba(0x1E1E1EC8), rgba(0x303030B0)],
        numerals: rgb(0xF4F4F4),
        hour_tick: rgba(0xF4F4F4C0),
        minute_tick: rgba(0xC8C8C870),
        hands: rgb(0xF6F6F6),
        seconds: rgb(0xA0A0A0),
        caption_fill: rgba(0xFFFFFF18),
    },
};

impl Palette {
    pub fn new(dark: bool, accent: Color, blurred: bool) -> Self {
        let palette = Self {
            accent,
            ..if dark { DARK } else { LIGHT }
        };
        if blurred {
            return palette;
        }
        Self {
            frost: palette.frost.solid(),
            panel: palette.panel.solid(),
            ..palette
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

impl Glass {
    fn solid(self) -> Self {
        Self {
            tint: Color {
                a: (self.tint.a + SOLID_BOOST).min(1.0),
                ..self.tint
            },
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_without_blur_is_more_opaque() {
        let accent = rgb(0x007AFF);
        let blurred = Palette::new(false, accent, true);
        let solid = Palette::new(false, accent, false);
        assert!(solid.frost.tint.a > blurred.frost.tint.a);
        assert!(solid.panel.tint.a > blurred.panel.tint.a);
    }
}
