//! 図の色と書体: tokens.json の表から `charta::Theme` を作り、シーンを SVG にする。
//!
//! 表は `build.rs` が `$OUT_DIR/chart_theme.rs` に写す（`theme.rs` の `chart_theme_rs`）。charta は値の源を
//! 読まず、ここで作った `Theme` を受け取る。

use charta::{Colors, Scene, Theme, to_svg};

mod table {
    include!(concat!(env!("OUT_DIR"), "/chart_theme.rs"));
}

/// ライトかダーク。tokens.json の `color.themes` の id と同じ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Light,
    Dark,
}

/// tokens.json の色（`mode` の値）と書体の `Theme`。
pub(crate) fn theme(mode: Mode) -> Theme {
    Theme::new(
        table::COLORS.iter().map(|(name, light, dark)| {
            (
                *name,
                match mode {
                    Mode::Light => *light,
                    Mode::Dark => *dark,
                },
            )
        }),
        table::FONT_SANS,
        table::FONT_MONO,
    )
}

/// 図の SVG。色は CSS 変数（`var(--color-ink)`）で書き、ライトとダークはページの CSS が切り替える
/// （`theme.css`）。図は 1 枚で、どちらのテーマでも同じ文字列。
pub(crate) fn svg(scene: &Scene) -> String {
    // CSS 変数の書き方では値を使わない。役割名と段数を確かめるためのテーマ
    to_svg(scene, &theme(Mode::Light), Colors::Variables).expect("役割名は tokens.json にある")
}

/// 継ぎ目: tokens.json の表 → `Theme`。表の値そのものは `theme.rs` の `chart_theme_rs` のテストが見る。
#[cfg(test)]
mod tests {
    use super::*;
    use charta::{Font, Item, Paint, Palette};

    #[test]
    fn themes_carry_the_light_and_dark_values_of_tokens_json() {
        let (light, dark) = (theme(Mode::Light), theme(Mode::Dark));
        assert_ne!(light.color("paper"), dark.color("paper"));
        assert_eq!(dark.color("chart-3"), dark.color("ink"));
        assert_eq!(light.color("chart-3"), light.color("ink"));
        assert_eq!(light.stops("seq"), 12);
        assert_eq!(light.stops("div"), 12);
        assert!(light.font(Font::Mono).contains("IBM Plex Mono"));
        assert!(light.font(Font::Sans).contains("IBM Plex Sans JP"));
    }

    #[test]
    fn the_figure_is_written_in_css_variables_only() {
        let mut scene = Scene::new(10.0, 10.0);
        scene.items.push(Item::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            fill: Some(Palette::Div.band_paint(3, 10)),
            stroke: None,
        });
        scene.items.push(Item::Text {
            x: 1.0,
            y: 1.0,
            text: "1".into(),
            size: 9.0,
            font: Font::Mono,
            fill: Paint::role("ink"),
            anchor: charta::Anchor::Middle,
            rotate: 0.0,
            halo: None,
        });
        let svg = svg(&scene);
        assert!(
            svg.contains("color-mix(in oklab, var(--color-div-"),
            "{svg}"
        );
        assert!(svg.contains("var(--color-ink)"), "{svg}");
        assert!(!svg.contains('#'), "値を含まない: {svg}");
    }
}
