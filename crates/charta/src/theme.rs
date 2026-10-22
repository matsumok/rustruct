//! 役割名 → 色と書体。値は `docs/design/tokens.json` にだけあり、`build.rs` が表に写す（`docs/stack.md`）。
//!
//! シーンは色を役割名（`ink`、`paper`、`seq-3`）で持ち、描くときにこの表で引く。ライトとダークは
//! 同じ役割名で値が入れ替わるだけ。

use crate::scene::Font;

mod tokens {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
}

/// ライトかダーク。tokens.json の `color.themes` の id と同じ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Light,
    Dark,
}

/// 役割名から色（`#rrggbb`）と書体（font-family の並び）を引く表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    mode: Mode,
}

impl Theme {
    pub fn new(mode: Mode) -> Self {
        Self { mode }
    }

    pub fn light() -> Self {
        Self::new(Mode::Light)
    }

    pub fn dark() -> Self {
        Self::new(Mode::Dark)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// 役割名の色。tokens.json に無い役割名は `None`。
    pub fn color(&self, role: &str) -> Option<&'static str> {
        tokens::COLORS
            .iter()
            .find(|(name, _, _)| *name == role)
            .map(|(_, light, dark)| match self.mode {
                Mode::Light => *light,
                Mode::Dark => *dark,
            })
    }

    /// 書体の font-family の並び（フォールバック込み）。
    pub fn font(&self, font: Font) -> &'static str {
        match font {
            Font::Sans => tokens::FONT_SANS,
            Font::Mono => tokens::FONT_MONO,
        }
    }

    /// tokens.json にある色の役割名をすべて。
    pub fn roles() -> impl Iterator<Item = &'static str> {
        tokens::COLORS.iter().map(|(name, _, _)| *name)
    }
}

/// 継ぎ目: tokens.json の値がそのまま引ける。期待値は `docs/design/tokens.json` に書いてある値。
#[cfg(test)]
mod tests {
    use super::*;

    fn is_hex(s: &str) -> bool {
        s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
    }

    #[test]
    fn roles_are_read_from_tokens_json() {
        let light = Theme::light();
        let dark = Theme::dark();
        for role in [
            "paper", "ink", "muted", "line", "seq-1", "seq-12", "div-1", "div-12",
        ] {
            let l = light.color(role).unwrap_or_else(|| panic!("{role} が無い"));
            let d = dark.color(role).unwrap_or_else(|| panic!("{role} が無い"));
            assert!(is_hex(l), "{role} のライトが #rrggbb でない: {l}");
            assert!(is_hex(d), "{role} のダークが #rrggbb でない: {d}");
        }
        assert_eq!(light.color("paper"), Some("#ffffff"));
        assert_ne!(light.color("paper"), dark.color("paper"));
        assert_eq!(light.color("no-such-role"), None);
    }

    #[test]
    fn references_are_resolved() {
        // chart-3 は `{ink}` の参照。値になって引ける
        let t = Theme::light();
        assert_eq!(t.color("chart-3"), t.color("ink"));
        assert!(Theme::roles().all(|r| is_hex(t.color(r).unwrap())));
    }

    #[test]
    fn fonts_are_the_token_families() {
        let t = Theme::light();
        assert!(t.font(Font::Mono).contains("IBM Plex Mono"));
        assert!(t.font(Font::Sans).contains("IBM Plex Sans JP"));
    }
}
