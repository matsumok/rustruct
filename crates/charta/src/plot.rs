//! 軸のある図の描く範囲。データの矩形と画面の矩形の対応と、枠・目盛り・軸の名前
//! （`docs/design/README.md`「グラフの色」の matplotlib 型）。
//!
//! - 描く範囲を `ink` の四辺の枠で囲む
//! - 目盛りは内向きで四辺に付け、数値は下と左だけ。数値と軸の名前は `muted`
//! - 格子は引かない

use crate::levels::{format_level, nice_step};
use crate::scene::{Anchor, Font, Item, Paint, Pt, Scene, Stroke};

/// 枠と目盛りの線の太さ。
pub const FRAME_WIDTH: f64 = 1.0;
/// 目盛りの長さ（枠から内へ）。
pub const TICK_LENGTH: f64 = 4.0;
/// 目盛りの数値と軸の名前の文字の大きさ（`label` の段）。図に重ねる名前や要約の文字も同じ大きさにする。
pub const TEXT_SIZE: f64 = 11.0;
/// 目盛りの数の目安（軸ごと）。刻みは 1・2・2.5・5 × 10ⁿ に丸める。
pub const TICK_STEPS: usize = 5;
/// 枠と目盛りの数値の間、目盛りの数値と軸の名前の間。
const GAP: f64 = 4.0;

/// データの矩形 `x`・`y`（最小、最大）を、画面の矩形（左上 `origin`、幅と高さ `size`）に写す。
/// データの y は上が正、画面の y は下が正。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plot {
    pub x: (f64, f64),
    pub y: (f64, f64),
    pub origin: Pt,
    pub size: [f64; 2],
}

impl Plot {
    /// データの点 → 画面の点。
    pub fn to_screen(&self, p: Pt) -> Pt {
        [
            self.origin[0] + (p[0] - self.x.0) / (self.x.1 - self.x.0) * self.size[0],
            self.origin[1] + self.size[1]
                - (p[1] - self.y.0) / (self.y.1 - self.y.0) * self.size[1],
        ]
    }

    /// 画面の矩形の左上と右下。
    pub fn bounds(&self) -> [Pt; 2] {
        [
            self.origin,
            [self.origin[0] + self.size[0], self.origin[1] + self.size[1]],
        ]
    }

    /// 枠の外に要る空き（左、上、右、下）。目盛りの数値と軸の名前が入る幅で、`axes` が描く位置と同じ。
    /// 数値は `digits` 文字まで入る。上と右は、枠の端の目盛りの数値が半分はみ出す分
    pub fn margins(digits: usize) -> [f64; 4] {
        let line = TEXT_SIZE + GAP;
        let number = number_width(digits);
        [
            GAP + number + GAP + line,
            TEXT_SIZE / 2.0,
            number / 2.0,
            GAP + line + line,
        ]
    }
}

/// 目盛りの数値 `digits` 文字の幅の見込み
fn number_width(digits: usize) -> f64 {
    Font::Mono.width(&"0".repeat(digits), TEXT_SIZE)
}

/// 枠、内向きの目盛り（四辺）、下と左の目盛りの数値、軸の名前。`digits` は数値の幅の見込み
/// （[`Plot::margins`] に渡したものと同じ）。
pub fn axes(plot: &Plot, x_name: &str, y_name: &str, digits: usize) -> Scene {
    let [lo, hi] = plot.bounds();
    let mut scene = Scene::new(hi[0], hi[1]);
    let mut ticks: Vec<Vec<Pt>> = Vec::new();
    let text = |x: f64, y: f64, s: String, font: Font, anchor: Anchor, rotate: f64| Item::Text {
        x,
        y,
        text: s,
        size: TEXT_SIZE,
        font,
        fill: Paint::role("muted"),
        anchor,
        rotate,
        halo: None,
    };
    let mut labels = Vec::new();

    let x_step = nice_step(plot.x.1 - plot.x.0, TICK_STEPS);
    for v in multiples(plot.x, x_step) {
        let sx = plot.to_screen([v, plot.y.0])[0];
        ticks.push(vec![[sx, hi[1]], [sx, hi[1] - TICK_LENGTH]]);
        ticks.push(vec![[sx, lo[1]], [sx, lo[1] + TICK_LENGTH]]);
        labels.push(text(
            sx,
            hi[1] + GAP + TEXT_SIZE / 2.0,
            format_level(v, x_step),
            Font::Mono,
            Anchor::Middle,
            0.0,
        ));
    }
    let y_step = nice_step(plot.y.1 - plot.y.0, TICK_STEPS);
    for v in multiples(plot.y, y_step) {
        let sy = plot.to_screen([plot.x.0, v])[1];
        ticks.push(vec![[lo[0], sy], [lo[0] + TICK_LENGTH, sy]]);
        ticks.push(vec![[hi[0], sy], [hi[0] - TICK_LENGTH, sy]]);
        labels.push(text(
            lo[0] - GAP,
            sy,
            format_level(v, y_step),
            Font::Mono,
            Anchor::End,
            0.0,
        ));
    }
    let line = TEXT_SIZE + GAP;
    labels.push(text(
        (lo[0] + hi[0]) / 2.0,
        hi[1] + GAP + line + TEXT_SIZE / 2.0,
        x_name.to_string(),
        Font::Sans,
        Anchor::Middle,
        0.0,
    ));
    labels.push(text(
        lo[0] - GAP - number_width(digits) - GAP - TEXT_SIZE / 2.0,
        (lo[1] + hi[1]) / 2.0,
        y_name.to_string(),
        Font::Sans,
        Anchor::Middle,
        -90.0,
    ));

    scene.items.push(Item::Rect {
        x: lo[0],
        y: lo[1],
        width: plot.size[0],
        height: plot.size[1],
        fill: None,
        stroke: Some(Stroke::new("ink", FRAME_WIDTH)),
    });
    scene.items.push(Item::Path {
        subpaths: ticks,
        closed: false,
        fill: None,
        stroke: Some(Stroke::new("ink", FRAME_WIDTH)),
    });
    scene.items.extend(labels);
    scene
}

/// 範囲 `(lo, hi)` の中の `step` の倍数（両端を含む。丸めの誤差は見逃す）。
fn multiples((lo, hi): (f64, f64), step: f64) -> impl Iterator<Item = f64> {
    let first = (lo / step - 1e-9).ceil() as i64;
    let last = (hi / step + 1e-9).floor() as i64;
    (first..=last).map(move |k| k as f64 * step)
}

/// 継ぎ目: 範囲 → 枠と目盛りのシーン。数値の並びと置き場所、色と書体。
#[cfg(test)]
mod tests {
    use super::*;

    fn plot() -> Plot {
        Plot {
            x: (3.0, 5.0),
            y: (4.0, 6.0),
            origin: [50.0, 10.0],
            size: [200.0, 200.0],
        }
    }

    fn texts(scene: &Scene) -> Vec<(f64, f64, &str, Font, f64)> {
        scene
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Text {
                    x,
                    y,
                    text,
                    font,
                    rotate,
                    fill,
                    size,
                    halo: None,
                    ..
                } => {
                    assert_eq!((fill, *size), (&Paint::role("muted"), TEXT_SIZE));
                    Some((*x, *y, text.as_str(), *font, *rotate))
                }
                Item::Text { .. } => panic!("白抜きの文字は無い: {item:?}"),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn data_corners_map_to_the_screen_corners_with_y_up() {
        let p = plot();
        assert_eq!(p.to_screen([3.0, 4.0]), [50.0, 210.0]);
        assert_eq!(p.to_screen([5.0, 6.0]), [250.0, 10.0]);
        assert_eq!(p.to_screen([4.0, 5.0]), [150.0, 110.0]);
    }

    #[test]
    fn frame_is_an_ink_rect_and_ticks_point_inward_on_all_four_sides() {
        let scene = axes(&plot(), "Lx（m）", "Ly（m）", 3);
        assert!(scene.items.contains(&Item::Rect {
            x: 50.0,
            y: 10.0,
            width: 200.0,
            height: 200.0,
            fill: None,
            stroke: Some(Stroke::new("ink", FRAME_WIDTH)),
        }));
        let ticks: Vec<&Vec<Pt>> = scene
            .items
            .iter()
            .flat_map(|item| match item {
                Item::Path {
                    subpaths, stroke, ..
                } => {
                    assert_eq!(stroke, &Some(Stroke::new("ink", FRAME_WIDTH)));
                    subpaths.iter().collect()
                }
                _ => vec![],
            })
            .collect();
        // 2 m を 5 つの目安で 0.5 刻み: 各軸 5 本、四辺で 20 本
        assert_eq!(ticks.len(), 20);
        for t in ticks {
            for p in t {
                assert!(
                    (50.0..=250.0).contains(&p[0]) && (10.0..=210.0).contains(&p[1]),
                    "枠の外: {p:?}"
                );
            }
            let length = (t[1][0] - t[0][0]).hypot(t[1][1] - t[0][1]);
            assert_eq!(length, TICK_LENGTH);
        }
    }

    #[test]
    fn numbers_are_on_the_bottom_and_left_and_names_under_and_beside() {
        let scene = axes(&plot(), "Lx（m）", "Ly（m）", 3);
        let t = texts(&scene);
        let bottom: Vec<&str> = t
            .iter()
            .filter(|(_, y, _, f, _)| *y > 210.0 && *f == Font::Mono)
            .map(|(.., s, _, _)| *s)
            .collect();
        assert_eq!(bottom, ["3.0", "3.5", "4.0", "4.5", "5.0"]);
        let left: Vec<&str> = t
            .iter()
            .filter(|(x, _, _, f, _)| *x < 50.0 && *f == Font::Mono)
            .map(|(.., s, _, _)| *s)
            .collect();
        assert_eq!(left, ["4.0", "4.5", "5.0", "5.5", "6.0"]);
        // 上と右には数値を書かない
        assert!(t.iter().all(|(x, y, ..)| *x <= 250.0 && *y >= 10.0));
        let names: Vec<_> = t.iter().filter(|(.., f, _)| *f == Font::Sans).collect();
        assert_eq!(names.len(), 2);
        let (_, by, _, _, _) = t.iter().find(|(.., s, _, _)| *s == "3.0").unwrap();
        let x_name = names.iter().find(|(.., s, _, _)| *s == "Lx（m）").unwrap();
        assert!(x_name.1 > *by && x_name.0 == 150.0 && x_name.4 == 0.0);
        let y_name = names.iter().find(|(.., s, _, _)| *s == "Ly（m）").unwrap();
        assert!(y_name.0 < 50.0 && y_name.1 == 110.0 && y_name.4 == -90.0);
        // 文字はすべて、枠の周りの空きの見込みの中に収まる（等幅の文字の幅は 0.6 字、縦の名前は回して見る）
        let [left, top, right, bottom] = Plot::margins(3);
        let (lo, hi) = ([50.0 - left, 10.0 - top], [250.0 + right, 210.0 + bottom]);
        for (x, y, s, font, rotate) in &t {
            let w =
                s.chars().count() as f64 * TEXT_SIZE * if *font == Font::Mono { 0.6 } else { 1.0 };
            let (w, h) = if *rotate == 0.0 {
                (w, TEXT_SIZE)
            } else {
                (TEXT_SIZE, w)
            };
            let x0 = if *font == Font::Mono && *x < 50.0 {
                x - w
            } else {
                x - w / 2.0
            };
            assert!(
                x0 >= lo[0] - 1e-9 && x0 + w <= hi[0] + 1e-9,
                "{s} が左右にはみ出す: {x0}〜{}",
                x0 + w
            );
            assert!(
                y - h / 2.0 >= lo[1] - 1e-9 && y + h / 2.0 <= hi[1] + 1e-9,
                "{s} が上下にはみ出す"
            );
        }
    }
}
