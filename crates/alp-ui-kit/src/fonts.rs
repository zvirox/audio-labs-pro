//! Bundled Inter typography.
//!
//! The variable font is shipped with the application so a missing system
//! package cannot change the interface. The OFL text lives next to the asset
//! and in `THIRD_PARTY.md`.

use iced::{Font, font};

/// Raw Inter variable font bytes loaded once by the application builder.
pub const INTER_BYTES: &[u8] = include_bytes!("../assets/fonts/InterVariable.ttf");

/// Family name used by every text widget.
pub const FAMILY: &str = "Inter Variable";

/// Default body font.
#[must_use]
pub fn regular() -> Font {
    Font {
        family: font::Family::Name(FAMILY),
        weight: font::Weight::Normal,
        ..Font::DEFAULT
    }
}

/// Emphasized label font.
#[must_use]
pub fn medium() -> Font {
    Font {
        weight: font::Weight::Medium,
        ..regular()
    }
}

/// Section and button font.
#[must_use]
pub fn semibold() -> Font {
    Font {
        weight: font::Weight::Semibold,
        ..regular()
    }
}

/// Display font.
#[must_use]
pub fn bold() -> Font {
    Font {
        weight: font::Weight::Bold,
        ..regular()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_font_is_present_and_named() {
        assert!(INTER_BYTES.len() > 100_000);
        assert_eq!(regular().family, font::Family::Name("Inter Variable"));
        assert_eq!(semibold().weight, font::Weight::Semibold);
    }
}
