//! 分布図: グリッドの値を等値線と帯で示す図を、`docs/adr/0011` で決めた見た目のシーンにする。
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
use crate::levels::{edges, format_level, step_of};
use crate::palette::Palette;
use crate::scene::{Anchor, Font, Item, Paint, Pt, Scene, Stroke};

/// 等値線の太さ。`docs/adr/0011`。
pub const LINE_WIDTH: f64 = 0.25;
/// 線の上の数値の文字の大きさ。文字の段（`label`・`body`）の外で、この数値だけに使う。`docs/adr/0011`。
pub const LABEL_SIZE: f64 = 9.0;
/// 白抜きの縁の太さ。`docs/adr/0011`。
pub const HALO_WIDTH: f64 = 2.5;
/// 等幅書体の 1 文字の送り（文字の大きさに対する比）。数値の幅の見積もりに使う。
const MONO_ADVANCE: f64 = 0.6;

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

    for label in labels(&screen_lines, step, width, height) {
        scene.items.push(Item::Text {
            x: label.at[0],
            y: label.at[1],
            text: label.text,
            size: LABEL_SIZE,
            font: Font::Mono,
            fill: Paint::role("ink"),
            anchor: Anchor::Middle,
            rotate: label.angle,
            halo: Some(Stroke::new("paper", HALO_WIDTH)),
        });
    }
    scene
}

/// 置いた数値。`at` は文字の中心、`angle` は度。
struct Label {
    at: Pt,
    angle: f64,
    text: String,
    /// 縁込みの大きさ（幅、高さ）。ぶつかりの判定に使う
    size: [f64; 2],
}

impl Label {
    /// 回転した矩形の 4 隅。
    fn corners(&self) -> [Pt; 4] {
        let (s, c) = self.angle.to_radians().sin_cos();
        let (hw, hh) = (self.size[0] / 2.0, self.size[1] / 2.0);
        [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]]
            .map(|[u, v]| [self.at[0] + u * c - v * s, self.at[1] + u * s + v * c])
    }
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

    let mut placed: Vec<Label> = Vec::new();
    for (_, at, angle, level) in crossings {
        let text = format_level(level, step);
        let label = Label {
            at,
            angle: upright(angle),
            size: [
                text.chars().count() as f64 * LABEL_SIZE * MONO_ADVANCE + HALO_WIDTH,
                LABEL_SIZE + HALO_WIDTH,
            ],
            text,
        };
        let corners = label.corners();
        let inside = corners
            .iter()
            .all(|p| p[0] >= 0.0 && p[0] <= width && p[1] >= 0.0 && p[1] <= height);
        if inside
            && placed
                .iter()
                .all(|other| !overlaps(&corners, &other.corners()))
        {
            placed.push(label);
        }
    }
    placed
}

/// 案内線 `a`→`b` と線分 `p`→`q` の交点。案内線の上の位置 `u`（0 で `a`、1 で `b`）、点、線分の向き（度）。
fn cross(a: Pt, b: Pt, p: Pt, q: Pt) -> Option<(f64, Pt, f64)> {
    let r = [q[0] - p[0], q[1] - p[1]];
    let g = [b[0] - a[0], b[1] - a[1]];
    let den = r[0] * g[1] - r[1] * g[0];
    if den.abs() < 1e-12 {
        return None;
    }
    let t = ((a[0] - p[0]) * g[1] - (a[1] - p[1]) * g[0]) / den;
    let u = ((a[0] - p[0]) * r[1] - (a[1] - p[1]) * r[0]) / den;
    if !(0.0..1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
        return None;
    }
    let at = [p[0] + t * r[0], p[1] + t * r[1]];
    Some((u, at, r[1].atan2(r[0]).to_degrees()))
}

/// 角度を [−90°, 90°) に収める（文字が逆さにならない。縦の線では下から上へ読む）。
fn upright(angle: f64) -> f64 {
    let mut a = angle;
    while a >= 90.0 {
        a -= 180.0;
    }
    while a < -90.0 {
        a += 180.0;
    }
    a
}

/// 2 つの凸四角形が重なるか（分離軸）。
fn overlaps(a: &[Pt; 4], b: &[Pt; 4]) -> bool {
    let axes = |r: &[Pt; 4]| {
        [
            [r[1][0] - r[0][0], r[1][1] - r[0][1]],
            [r[3][0] - r[0][0], r[3][1] - r[0][1]],
        ]
    };
    for axis in axes(a).into_iter().chain(axes(b)) {
        let project = |r: &[Pt; 4]| {
            r.iter()
                .map(|p| p[0] * axis[0] + p[1] * axis[1])
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                    (lo.min(v), hi.max(v))
                })
        };
        let (a0, a1) = project(a);
        let (b0, b1) = project(b);
        if a1 < b0 || b1 < a0 {
            return false;
        }
    }
    true
}

/// 継ぎ目: グリッド → 決めた見た目のシーン。帯・線・数値の形と、数値が十字に並ぶこと。
#[cfg(test)]
mod tests {
    use super::*;
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
                Item::Rect { .. } => panic!("分布図に矩形は無い"),
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
            texts.contains(&"0") && texts.contains(&"\u{2212}8") && texts.contains(&"8"),
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

    #[test]
    fn upright_keeps_text_from_being_upside_down() {
        assert_eq!(upright(135.0), -45.0);
        assert_eq!(upright(-135.0), 45.0);
        assert_eq!(upright(90.0), -90.0);
        assert_eq!(upright(-90.0), -90.0);
        assert_eq!(upright(180.0), 0.0);
    }
}
