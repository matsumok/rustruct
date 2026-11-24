//! 図の色と書体: tokens.json の表から `charta::Theme` を作り、シーンを SVG にする。
//!
//! 表は `build.rs` が `$OUT_DIR/chart_theme.rs` に写す（`theme.rs` の `chart_theme_rs`）。charta は値の源を
//! 読まず、ここで作った `Theme` を受け取る。図の module は `Scene` までを作り、SVG にするのはページが呼ぶ
//! [`svg`] だけ。

use charta::{Colors, Scene, Theme, to_svg};

mod table {
    include!(concat!(env!("OUT_DIR"), "/chart_theme.rs"));
}

/// tokens.json の色の役割名と書体の `Theme`。色は CSS 変数で書くので値は使わず、役割名と段数を確かめる
/// ためだけにある（値はライトのもの）。
fn theme() -> Theme {
    Theme::new(
        table::COLORS.iter().map(|(name, light, _)| (*name, *light)),
        table::FONT_SANS,
        table::FONT_MONO,
    )
}

/// 図の SVG。色は CSS 変数（`var(--color-ink)`）で書き、ライトとダークはページの CSS が切り替える
/// （`theme.css`）。図は 1 枚で、どちらのテーマでも同じ文字列。
pub(crate) fn svg(scene: &Scene) -> String {
    to_svg(scene, &theme(), Colors::Variables).expect("役割名は tokens.json にある")
}

/// 継ぎ目: tokens.json の表 → `Theme` と、シーン → SVG の文字の形。表の値そのものは `theme.rs` の `chart_theme_rs` のテストが見る。
#[cfg(test)]
mod tests {
    use super::*;
    use charta::{Font, Item, Paint, Palette, Stroke};

    #[test]
    fn the_theme_carries_the_roles_and_fonts_of_tokens_json() {
        let theme = theme();
        assert_eq!(theme.color("chart-3"), theme.color("ink"));
        assert_eq!(theme.stops("seq"), 12);
        assert_eq!(theme.stops("div"), 12);
        assert!(theme.font(Font::Mono).contains("IBM Plex Mono"));
        assert!(theme.font(Font::Sans).contains("IBM Plex Sans JP"));
    }

    #[test]
    fn the_figure_is_written_in_css_variables_only() {
        // 図の 4 種（帯と線のパス、枠、印、白抜きの文字）。帯の色は段の色そのものか、隣り合う 2 段を
        // OKLab で混ぜた色
        let mut scene = Scene::new(10.0, 10.0);
        scene.items.push(Item::Path {
            subpaths: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]],
            closed: true,
            fill: Some(Palette::Seq.band_paint(0, 12)),
            stroke: None,
        });
        scene.items.push(Item::Path {
            subpaths: vec![vec![[0.0, 0.0], [1.0, 1.0]]],
            closed: false,
            fill: None,
            stroke: Some(Stroke::new("ink", 0.25)),
        });
        scene.items.push(Item::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            fill: Some(Palette::Div.band_paint(3, 10)),
            stroke: Some(Stroke::new("ink", 1.0)),
        });
        scene.items.push(Item::Circle {
            center: [5.0, 5.0],
            radius: 3.5,
            fill: Some(Paint::role("paper")),
            stroke: Some(Stroke::new("ink", 1.0)),
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
            halo: Some(Stroke::new("paper", 2.5)),
        });
        let svg = svg(&scene);
        assert!(svg.contains("var(--color-seq-"), "{svg}");
        assert!(
            svg.contains("color-mix(in oklab, var(--color-div-"),
            "{svg}"
        );
        assert!(svg.contains("var(--color-ink)"), "{svg}");
        assert!(svg.contains("var(--color-paper)"), "{svg}");
        assert!(!svg.contains('#'), "値を含まない: {svg}");
        // 色は style の中だけに、CSS 変数か、その混ぜ合わせか、無しで書く
        assert!(
            !svg.contains(" fill=\"") && !svg.contains(" stroke=\""),
            "{svg}"
        );
        let styles: Vec<&str> = svg
            .split(" style=\"")
            .skip(1)
            .map(|rest| &rest[..rest.find('"').unwrap()])
            .collect();
        assert!(!styles.is_empty(), "{svg}");
        for declaration in styles.iter().flat_map(|style| style.split("; ")) {
            let (_, value) = declaration.split_once(": ").unwrap();
            assert!(
                ["none", "var(--color-", "color-mix(in oklab, var(--color-"]
                    .iter()
                    .any(|head| value.starts_with(head)),
                "{declaration}"
            );
        }
    }
}
