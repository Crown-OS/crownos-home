//! Colours as settings files spell them.

use geometry::Color;

/// `#RRGGBB` or `#RRGGBBAA`.
pub fn parse_hex(text: &str) -> Option<Color> {
    let digits = text.strip_prefix('#')?;
    if !matches!(digits.len(), 6 | 8) || !digits.is_ascii() {
        return None;
    }
    let channel = |index: usize| u8::from_str_radix(digits.get(index..index + 2)?, 16).ok();
    let alpha = if digits.len() == 8 { channel(6)? } else { 255 };
    Some(Color::from_rgba8(
        channel(0)?,
        channel(2)?,
        channel(4)?,
        alpha,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_and_eight_digit_colours_parse() {
        assert_eq!(
            parse_hex("#7B6EFD"),
            Some(Color::from_rgba8(0x7B, 0x6E, 0xFD, 255))
        );
        assert_eq!(
            parse_hex("#00000080"),
            Some(Color::from_rgba8(0, 0, 0, 0x80))
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for text in ["violet", "#123", "7B6EFD", "#GG0000"] {
            assert_eq!(parse_hex(text), None, "{text}");
        }
    }
}
