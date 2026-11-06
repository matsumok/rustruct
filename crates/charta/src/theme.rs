//! 役割名 → 色と書体。値は呼び手が持ち、[`Theme::new`] で渡す（charta は値の源を読まない）。
//!
//! シーンは色を役割名（`ink`、`paper`、`seq-3`）で持ち、描くときにこの表で引く。ライトとダークを
//! 描き分けるなら、呼び手が同じ役割名で値の違う表を 2 つ作る。

use crate::scene::Font;

/// 役割名から色（`#rrggbb`）と書体（font-family の並び）を引く表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// (役割名, 値)。並びは渡された順
    colors: Vec<(String, String)>,
    sans: String,
    mono: String,
}

impl Theme {
    /// 色の (役割名, 値) の並びと、2 つの書体の font-family の並び（フォールバック込み）。
    pub fn new<R: Into<String>, V: Into<String>>(
        colors: impl IntoIterator<Item = (R, V)>,
        sans: impl Into<String>,
        mono: impl Into<String>,
    ) -> Self {
        Self {
            colors: colors
                .into_iter()
                .map(|(r, v)| (r.into(), v.into()))
                .collect(),
            sans: sans.into(),
            mono: mono.into(),
        }
    }

    /// 役割名の色。表に無い役割名は `None`。
    pub fn color(&self, role: &str) -> Option<&str> {
        self.colors
            .iter()
            .find(|(name, _)| name == role)
            .map(|(_, value)| value.as_str())
    }

    /// 書体の font-family の並び（フォールバック込み）。
    pub fn font(&self, font: Font) -> &str {
        match font {
            Font::Sans => &self.sans,
            Font::Mono => &self.mono,
        }
    }

    /// 表にある色の役割名をすべて、渡された順に。
    pub fn roles(&self) -> impl Iterator<Item = &str> {
        self.colors.iter().map(|(name, _)| name.as_str())
    }

    /// 色の列 `{prefix}-1`、`{prefix}-2` … が 1 から途切れずに何段あるか。
    pub fn stops(&self, prefix: &str) -> usize {
        (1..)
            .take_while(|k| self.color(&format!("{prefix}-{k}")).is_some())
            .count()
    }
}

/// テストと例の小さな表。値は `#rrggbb` で、段は seq も div も 12。
#[cfg(test)]
pub(crate) fn sample() -> Theme {
    let mut colors: Vec<(String, String)> = [
        ("paper", "#ffffff"),
        ("ink", "#1c2024"),
        ("muted", "#60646c"),
        ("fill", "#f0f0f3"),
    ]
    .map(|(r, v)| (r.to_string(), v.to_string()))
    .into();
    for k in 1..=12 {
        let v = (k * 20) as u8;
        colors.push((format!("seq-{k}"), format!("#{v:02x}{v:02x}ff")));
        colors.push((format!("div-{k}"), format!("#ff{v:02x}{v:02x}")));
    }
    Theme::new(
        colors,
        r#""Sample Sans", sans-serif"#,
        r#""Sample Mono", monospace"#,
    )
}

/// 継ぎ目: 呼び手の渡した表から引ける。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_and_fonts_are_the_ones_given() {
        let t = Theme::new(
            [("paper", "#ffffff"), ("ink", "#1c2024")],
            "\"A\", sans-serif",
            "\"B\", monospace",
        );
        assert_eq!(t.color("paper"), Some("#ffffff"));
        assert_eq!(t.color("ink"), Some("#1c2024"));
        assert_eq!(t.color("no-such-role"), None);
        assert_eq!(t.font(Font::Sans), "\"A\", sans-serif");
        assert_eq!(t.font(Font::Mono), "\"B\", monospace");
        assert_eq!(t.roles().collect::<Vec<_>>(), ["paper", "ink"]);
    }

    #[test]
    fn stops_count_the_unbroken_run_from_one() {
        let t = Theme::new(
            [
                ("seq-1", "#000000"),
                ("seq-2", "#111111"),
                ("seq-3", "#222222"),
                ("seq-5", "#444444"),
                ("div-2", "#000000"),
            ],
            "",
            "",
        );
        assert_eq!(t.stops("seq"), 3);
        assert_eq!(t.stops("div"), 0);
        assert_eq!(sample().stops("seq"), 12);
        assert_eq!(sample().stops("div"), 12);
    }
}
