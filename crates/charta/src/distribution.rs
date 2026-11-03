//! 分布図: グリッドの値を等値線と帯で示す図を、ADR site-0011 で決めた見た目のシーンにする。
//!
//! - 帯は `seq`（正負のない量）か `div`（正負のある量）。境は 1・2・2.5・5 × 10ⁿ の刻み
//! - 等値線は `ink` の細線。0 の線も同じ
//! - 線の上の数値は等幅の小さな文字を `ink` で書き、`paper` の縁で白抜きにする。線に沿って傾け、
//!   上下が逆にならないように ±90° に収める
//! - 数値の位置は、板の中心から四辺の中点へ引いた案内線（描かない）と等値線の交点。隣とぶつかれば
//!   中心に近い方を残す。案内線と交わらない線には付けない
//!
//! カラーバーはまだ決まっていないので描かない（境は `levels::edges` で同じものが得られる）。

use crate::contour::contour;
use crate::grid::Grid;
use crate::label::{Candidate, Label, cross, place};
use crate::levels::{edges, format_level, step_of};
use crate::palette::Palette;
use crate::scene::{Item, Pt, Scene, Stroke};

/// 等値線の太さ。ADR site-0011。
pub const LINE_WIDTH: f64 = 0.25;
pub use crate::label::{HALO_WIDTH, LABEL_SIZE};

/// グリッドの値を `width` の幅の図にする。高さはグリッドの縦横比から導く。
/// `range` は帯の境を決める値の範囲（最小、最大）で、ふつうは `Grid::range`。範囲の外の値は両端の帯の色で
/// 振り切れ、等値線は引かない。`steps` は帯の段数の目安（`Palette::default_steps` が既定）。
pub fn distribution(
    grid: &Grid,
    palette: Palette,
    range: (f64, f64),
    steps: usize,
    width: f64,
) -> Scene {
    let (x0, x1, y0, y1) = grid.extent();
    let height = width * (y1 - y0) / (x1 - x0);
    let to_screen = |p: Pt| -> Pt {
        [
            (p[0] - x0) / (x1 - x0) * width,
            height - (p[1] - y0) / (y1 - y0) * height,
        ]
    };

    let edges = edges(palette, range.0, range.1, steps);
    let step = step_of(&edges);
    let levels = &edges[1..edges.len() - 1];
    let (isolines, bands) = contour(grid, levels);

    let mut scene = Scene::new(width, height);
    for (k, band) in bands.iter().enumerate() {
        if band.polygons.is_empty() {
            continue;
        }
        scene.items.push(Item::Path {
            subpaths: band
                .polygons
                .iter()
                .map(|poly| poly.iter().map(|&p| to_screen(p)).collect())
                .collect(),
            closed: true,
            fill: Some(palette.band_paint(k, bands.len())),
            stroke: None,
        });
    }

    let mut screen_lines: Vec<(f64, Vec<Pt>)> = Vec::new();
    for iso in &isolines {
        if iso.lines.is_empty() {
            continue;
        }
        let lines: Vec<Vec<Pt>> = iso
            .lines
            .iter()
            .map(|line| line.iter().map(|&p| to_screen(p)).collect())
            .collect();
        scene.items.push(Item::Path {
            subpaths: lines.clone(),
            closed: false,
            fill: None,
            stroke: Some(Stroke::new("ink", LINE_WIDTH)),
        });
        screen_lines.extend(lines.into_iter().map(|l| (iso.level, l)));
    }

    scene.items.extend(
        labels(&screen_lines, step, width, height)
            .into_iter()
            .map(Label::into_item),
    );
    scene
}

/// 案内線（中心から四辺の中点へ）と等値線の交点に、中心に近い順に、ぶつからないものだけ置く。
fn labels(lines: &[(f64, Vec<Pt>)], step: f64, width: f64, height: f64) -> Vec<Label> {
    let center = [width / 2.0, height / 2.0];
    let ends = [
        [0.0, center[1]],
        [width, center[1]],
        [center[0], 0.0],
        [center[0], height],
    ];
    let mut crossings: Vec<(f64, Pt, f64, f64)> = Vec::new();
    for end in ends {
        for (level, line) in lines {
            for w in line.windows(2) {
                if let Some((u, p, angle)) = cross(center, end, w[0], w[1]) {
                    crossings.push((u, p, angle, *level));
                }
            }
        }
    }
    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
    place(
        crossings
            .into_iter()
            .map(|(_, at, angle, level)| Candidate {
                at,
                angle,
                text: format_level(level, step),
            }),
        [[0.0, 0.0], [width, height]],
        &[],
    )
}

/// 継ぎ目: グリッド → 決めた見た目のシーン。帯・線・数値の形と、数値が十字に並ぶこと。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Anchor, Font, Paint};
    use crate::svg::to_svg;
    use crate::theme::Theme;
    use std::f64::consts::PI;

    /// 2 × 3 の板の山（最大 7.3）。
    fn bump() -> Grid {
        Grid::from_fn(Grid::axis(2.0, 20), Grid::axis(3.0, 30), |x, y| {
            7.3 * (PI * x / 2.0).sin() * (PI * y / 3.0).sin()
        })
        .unwrap()
    }

    /// 2 × 2 の板の、x の中央で符号が変わる場（±10）。
    fn signed() -> Grid {
        Grid::from_fn(Grid::axis(2.0, 20), Grid::axis(2.0, 20), |x, y| {
            10.0 * (PI * x / 2.0).cos() * (PI * y / 2.0).sin()
        })
        .unwrap()
    }

    struct Parts<'a> {
        bands: Vec<&'a Item>,
        lines: Vec<&'a Item>,
        labels: Vec<&'a Item>,
    }

    fn parts(scene: &Scene) -> Parts<'_> {
        let mut p = Parts {
            bands: vec![],
            lines: vec![],
            labels: vec![],
        };
        for item in &scene.items {
            match item {
                Item::Path { closed: true, .. } => p.bands.push(item),
                Item::Path { closed: false, .. } => p.lines.push(item),
                Item::Text { .. } => p.labels.push(item),
                Item::Rect { .. } | Item::Circle { .. } => panic!("分布図に矩形と円は無い"),
            }
        }
        p
    }

    #[test]
    fn seq_figure_has_bands_thin_ink_lines_and_outlined_labels_on_the_cross() {
        let scene = distribution(&bump(), Palette::Seq, bump().range(), 10, 200.0);
        assert_eq!((scene.width, scene.height), (200.0, 300.0));
        let p = parts(&scene);

        // 帯: 0〜8 を 1 刻みで 8 本。すべて seq の間の色。描く順は帯 → 線 → 数値
        assert_eq!(p.bands.len(), 8);
        for band in &p.bands {
            let Item::Path {
                fill: Some(Paint::Blend { from, .. }),
                stroke: None,
                ..
            } = band
            else {
                panic!("帯の形が違う: {band:?}");
            };
            assert!(from.starts_with("seq-"));
        }
        let kind = |item: &Item| match item {
            Item::Path { closed: true, .. } => 0,
            Item::Path { .. } => 1,
            _ => 2,
        };
        assert!(scene.items.windows(2).all(|w| kind(&w[0]) <= kind(&w[1])));

        // 線: 水準 1〜7 の 7 本、ink の細線
        assert_eq!(p.lines.len(), 7);
        for line in &p.lines {
            let Item::Path {
                fill: None,
                stroke: Some(stroke),
                ..
            } = line
            else {
                panic!("線の形が違う: {line:?}");
            };
            assert_eq!(stroke, &Stroke::new("ink", LINE_WIDTH));
        }

        // 数値: 等幅の小さな文字、ink、paper の縁。十字の上。±90° の中。重ならない
        assert!(!p.labels.is_empty());
        let mut texts = vec![];
        for label in &p.labels {
            let Item::Text {
                x,
                y,
                text,
                size,
                font: Font::Mono,
                fill,
                anchor: Anchor::Middle,
                rotate,
                halo,
            } = label
            else {
                panic!("数値の形が違う: {label:?}");
            };
            assert_eq!(
                (*size, fill, halo),
                (
                    LABEL_SIZE,
                    &Paint::role("ink"),
                    &Some(Stroke::new("paper", HALO_WIDTH))
                )
            );
            assert!(
                (*x - 100.0).abs() < 1e-9 || (*y - 150.0).abs() < 1e-9,
                "十字の外: ({x}, {y})"
            );
            assert!(*rotate >= -90.0 && *rotate < 90.0);
            assert!(*x >= 0.0 && *x <= 200.0 && *y >= 0.0 && *y <= 300.0);
            texts.push(text.clone());
        }
        // 山は対称なので、各水準の数値は縦横に 2 つずつ（中心に近い順に残る）
        for t in ["1", "2", "3", "4", "5", "6", "7"] {
            assert!(
                texts.iter().filter(|s| *s == t).count() >= 2,
                "{t} が足りない: {texts:?}"
            );
        }
        for (i, a) in p.labels.iter().enumerate() {
            for b in &p.labels[..i] {
                let (Item::Text { x: ax, y: ay, .. }, Item::Text { x: bx, y: by, .. }) = (a, b)
                else {
                    unreachable!()
                };
                assert!(
                    (ax - bx).hypot(ay - by) >= LABEL_SIZE,
                    "重なる: {a:?} {b:?}"
                );
            }
        }
    }

    #[test]
    fn div_figure_splits_at_zero_and_draws_the_zero_line_like_the_others() {
        let scene = distribution(&signed(), Palette::Div, signed().range(), 16, 240.0);
        let p = parts(&scene);
        // ±10 を 16 段の目安で 2 刻み: -10〜10 の 10 本。負は div-1〜6、正は div-7〜12
        assert_eq!(p.bands.len(), 10);
        let froms: Vec<&str> = p
            .bands
            .iter()
            .map(|b| match b {
                Item::Path {
                    fill: Some(Paint::Blend { from, .. }),
                    ..
                } => from.as_str(),
                _ => unreachable!(),
            })
            .collect();
        assert!(
            froms[..5]
                .iter()
                .all(|f| ["div-1", "div-2", "div-3", "div-4", "div-5"].contains(f)),
            "{froms:?}"
        );
        assert!(
            froms[5..]
                .iter()
                .all(|f| ["div-7", "div-8", "div-9", "div-10", "div-11"].contains(f)),
            "{froms:?}"
        );
        // 0 の線もほかと同じ太さ
        assert_eq!(p.lines.len(), 9);
        assert!(
            p.lines
                .iter()
                .all(|l| matches!(l, Item::Path { stroke: Some(s), .. } if s.width == LINE_WIDTH))
        );
        let texts: Vec<&str> = p
            .labels
            .iter()
            .map(|l| match l {
                Item::Text { text, .. } => text.as_str(),
                _ => unreachable!(),
            })
            .collect();
        assert!(
            texts.contains(&"0") && texts.contains(&"-8") && texts.contains(&"8"),
            "{texts:?}"
        );
    }

    #[test]
    fn flat_grid_is_one_band_without_lines_or_labels() {
        let grid = Grid::from_fn(Grid::axis(1.0, 4), Grid::axis(1.0, 4), |_, _| 0.0).unwrap();
        let scene = distribution(&grid, Palette::Seq, grid.range(), 10, 100.0);
        let p = parts(&scene);
        assert_eq!((p.bands.len(), p.lines.len(), p.labels.len()), (1, 0, 0));
    }

    #[test]
    fn values_beyond_the_range_take_the_color_of_the_end_band() {
        // 0〜10 の傾き。色の幅を 0〜5 にすると、等値線は 5 未満だけに引かれ、5 を超える側は最後の帯の色
        let grid = Grid::from_fn(Grid::axis(1.0, 20), Grid::axis(1.0, 4), |x, _| 10.0 * x).unwrap();
        let scene = distribution(&grid, Palette::Seq, (0.0, 5.0), 10, 100.0);
        let p = parts(&scene);
        let numbers: Vec<f64> = p
            .labels
            .iter()
            .map(|l| match l {
                Item::Text { text, .. } => text.parse().unwrap(),
                _ => unreachable!(),
            })
            .collect();
        assert!(numbers.contains(&4.5), "{numbers:?}");
        assert!(numbers.iter().all(|v| *v < 5.0), "{numbers:?}");
        let Some(Item::Path { subpaths, .. }) = p.bands.last() else {
            panic!("帯が無い");
        };
        let right = subpaths.iter().flatten().map(|q| q[0]).fold(0.0, f64::max);
        assert_eq!(right, 100.0);
    }

    #[test]
    fn figure_renders_in_both_themes() {
        for grid in [bump(), signed()] {
            for palette in [Palette::Seq, Palette::Div] {
                let scene =
                    distribution(&grid, palette, grid.range(), palette.default_steps(), 300.0);
                for theme in [Theme::light(), Theme::dark()] {
                    let svg = to_svg(&scene, &theme).unwrap();
                    assert!(svg.contains("paint-order=\"stroke\""));
                    assert!(svg.contains("stroke-width=\"0.25\""));
                }
            }
        }
    }
}
