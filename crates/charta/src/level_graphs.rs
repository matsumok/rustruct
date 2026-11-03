//! 水準の境がグラフ y = g(x) の形で与えられる場の、等値線と帯の図。
//!
//! 場は x にも y にも単調に増える（必要スラブ厚の (Lx, Ly) の場）。水準ごとに、x の並びの上で境の y を
//! 受け取り、グリッドを解かずにそのまま線と帯にする（slab-thickness の ADR-0001）。境の下は水準より小さく、
//! 上は水準以上。見た目は分布図（`docs/adr/0011`）にそろえる:
//!
//! - 帯は隣り合う水準の境の間。色は `seq` で、`color_step` ごとの段（境が `color_step` の倍数のところで
//!   色が変わる）。最初の水準の境の下は塗らない
//! - 線は `ink` の細線（分布図と同じ太さ）
//! - 線の上の数値は分布図と同じ白抜きの文字。場が両方向に増えるので、描く範囲の左下から右上への対角線は
//!   範囲に現れる水準の境とちょうど一度ずつ交わる。数値はその交点に置き、ぶつかるときは `color_step` の
//!   倍数の水準（色の変わり目）を先に、次に範囲の中心に近いものを残す。交点が図の上の印や文字（`clear`）に
//!   かかるか範囲からはみ出すときは、同じ線の上を交点に近い順に、置ける場所を探す

use crate::distribution::LINE_WIDTH;
use crate::label::{Candidate, LABEL_SIZE, cross, place_groups};
use crate::levels::format_level;
use crate::palette::Palette;
use crate::plot::Plot;
use crate::scene::{Item, Pt, Scene, Stroke};

/// 1 つの水準の境。`ys[i]` は x の並びの `i` 番目での境の y。`None` はその x に境が無いこと
/// （どの y でも値が水準に届かない。境は上の無限遠にある）。
#[derive(Debug, Clone, PartialEq)]
pub struct LevelGraph {
    pub level: f64,
    pub ys: Vec<Option<f64>>,
}

/// `graphs` は水準の昇順。`xs` は昇順で、描く範囲の x の中にあり、両端を範囲の端に置く（帯がそこまで届く）。
/// 線の上の数値の桁は、最後の 2 つの水準の差を刻みとして決める。`clear` は数値を置かない画面の矩形
/// （左上と右下）で、図の上に重ねる印や文字の場所
pub fn level_graphs(
    plot: &Plot,
    xs: &[f64],
    graphs: &[LevelGraph],
    color_step: f64,
    clear: &[[Pt; 2]],
) -> Scene {
    let [_, corner] = plot.bounds();
    let mut scene = Scene::new(corner[0], corner[1]);
    let (Some(first), Some(last)) = (graphs.first(), graphs.last()) else {
        return scene;
    };

    // 帯。色の段は color_step の倍数で区切る
    let base = (first.level / color_step + 1e-9).floor() * color_step;
    let shades = (((last.level - base) / color_step - 1e-9).ceil() as usize).max(1);
    let clamp = |y: Option<f64>| y.unwrap_or(f64::INFINITY).clamp(plot.y.0, plot.y.1);
    for pair in graphs.windows(2) {
        let (lower, upper) = (&pair[0], &pair[1]);
        let bottom: Vec<f64> = lower.ys.iter().map(|&y| clamp(y)).collect();
        let top: Vec<f64> = upper.ys.iter().map(|&y| clamp(y)).collect();
        if bottom.iter().zip(&top).all(|(b, t)| t <= b) {
            continue;
        }
        let polygon = simplify(
            xs.iter()
                .zip(&bottom)
                .map(|(&x, &y)| plot.to_screen([x, y]))
                .chain(
                    xs.iter()
                        .zip(&top)
                        .rev()
                        .map(|(&x, &y)| plot.to_screen([x, y])),
                )
                .collect(),
        );
        let shade = (((lower.level - base) / color_step + 1e-9).floor() as usize).min(shades - 1);
        scene.items.push(Item::Path {
            subpaths: vec![polygon],
            closed: true,
            fill: Some(Palette::Seq.band_paint(shade, shades)),
            stroke: None,
        });
    }

    // 線。境の無い x で切り、描く範囲で切り取る
    let mut screen_lines: Vec<(f64, Vec<Pt>)> = Vec::new();
    for graph in graphs {
        let mut runs: Vec<Vec<Pt>> = Vec::new();
        for (a, b) in xs
            .iter()
            .zip(&graph.ys)
            .zip(xs.iter().zip(&graph.ys).skip(1))
        {
            let (Some(ya), Some(yb)) = (a.1, b.1) else {
                continue;
            };
            let Some((p, q)) = clip(plot, [*a.0, *ya], [*b.0, *yb]) else {
                continue;
            };
            let (p, q) = (plot.to_screen(p), plot.to_screen(q));
            match runs.last_mut() {
                Some(run) if run.last() == Some(&p) => run.push(q),
                _ => runs.push(vec![p, q]),
            }
        }
        if runs.is_empty() {
            continue;
        }
        scene.items.push(Item::Path {
            subpaths: runs.clone(),
            closed: false,
            fill: None,
            stroke: Some(Stroke::new("ink", LINE_WIDTH)),
        });
        screen_lines.extend(runs.into_iter().map(|run| (graph.level, run)));
    }

    // 数値。左下から右上への対角線との交点
    let step = match graphs {
        [.., a, b] => b.level - a.level,
        _ => color_step,
    };
    let from = plot.to_screen([plot.x.0, plot.y.0]);
    let to = plot.to_screen([plot.x.1, plot.y.1]);
    let is_edge = |level: f64| {
        let k = (level / color_step).round();
        (k * color_step - level).abs() < 1e-6 * color_step
    };
    // 対角線は水準ごとに一度だけ交わる。交点が線分の継ぎ目に来ると丸めで両側の線分が拾うので、水準ごとに
    // 最初の 1 つ
    let mut crossings: Vec<(f64, Pt, f64, f64)> = Vec::new();
    for (level, line) in &screen_lines {
        for w in line.windows(2) {
            if let Some((u, at, angle)) = cross(from, to, w[0], w[1])
                && !crossings.iter().any(|c| c.3 == *level)
            {
                crossings.push((u, at, angle, *level));
            }
        }
    }
    crossings.sort_by(|a, b| {
        (!is_edge(a.3))
            .cmp(&!is_edge(b.3))
            .then((a.0 - 0.5).abs().total_cmp(&(b.0 - 0.5).abs()))
    });
    // 交点が印や文字にかかるときは、同じ線の上を交点から近い順に
    let groups = crossings.into_iter().map(|(_, at, angle, level)| {
        let text = format_level(level, step);
        let mut others: Vec<(Pt, f64)> = screen_lines
            .iter()
            .filter(|(l, _)| *l == level)
            .flat_map(|(_, run)| along(run, ALONG_STEP))
            .collect();
        others.sort_by(|a, b| distance(a.0, at).total_cmp(&distance(b.0, at)));
        std::iter::once((at, angle))
            .chain(others)
            .map(|(at, angle)| Candidate {
                at,
                angle,
                text: text.clone(),
            })
            .collect::<Vec<_>>()
    });
    scene.items.extend(
        place_groups(groups, plot.bounds(), clear)
            .into_iter()
            .map(|label| label.into_item()),
    );
    scene
}

/// 交点から外したときに、線の上で次の候補を探す間隔（線に沿った長さ）。数値の文字の高さの半分
const ALONG_STEP: f64 = LABEL_SIZE / 2.0;

fn distance(a: Pt, b: Pt) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// 折れ線 `run` の上に、始点から線に沿って `spacing` ごとに取った点と、そこでの線の向き（度）
fn along(run: &[Pt], spacing: f64) -> Vec<(Pt, f64)> {
    let mut out = Vec::new();
    // 次の点までに残る線に沿った長さ
    let mut ahead = 0.0;
    for w in run.windows(2) {
        let (p, q) = (w[0], w[1]);
        let length = distance(p, q);
        if length == 0.0 {
            continue;
        }
        let angle = (q[1] - p[1]).atan2(q[0] - p[0]).to_degrees();
        let mut s = ahead;
        while s < length {
            let t = s / length;
            out.push(([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])], angle));
            s += spacing;
        }
        ahead = s - length;
    }
    out
}

/// 帯の輪郭から、続く同じ点と、水平に並ぶ点の間の点を除く（帯が枠の上下の辺に沿う所で点が並ぶ）
fn simplify(points: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(points.len());
    for p in points {
        if out.last() == Some(&p) {
            continue;
        }
        if let [.., a, b] = out[..]
            && a[1] == b[1]
            && b[1] == p[1]
        {
            out.pop();
        }
        out.push(p);
    }
    out
}

/// 線分 `p`→`q` を描く範囲（データの矩形）で切り取る（Liang–Barsky）。範囲を通らなければ `None`。
fn clip(plot: &Plot, p: Pt, q: Pt) -> Option<(Pt, Pt)> {
    let d = [q[0] - p[0], q[1] - p[1]];
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (denom, num) in [
        (-d[0], p[0] - plot.x.0),
        (d[0], plot.x.1 - p[0]),
        (-d[1], p[1] - plot.y.0),
        (d[1], plot.y.1 - p[1]),
    ] {
        if denom == 0.0 {
            if num < 0.0 {
                return None;
            }
        } else {
            let t = num / denom;
            if denom < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t0 < t1).then(|| {
        let at = |t: f64| [p[0] + t * d[0], p[1] + t * d[1]];
        (at(t0), at(t1))
    })
}

/// 継ぎ目: 境のグラフ → 帯・線・数値のシーン。場は v = 10·(x + y)、境は y = level/10 − x。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Paint;

    fn plot() -> Plot {
        Plot {
            x: (0.0, 10.0),
            y: (0.0, 10.0),
            origin: [0.0, 0.0],
            size: [200.0, 200.0],
        }
    }

    fn xs() -> Vec<f64> {
        (0..=100).map(|i| i as f64 * 0.1).collect()
    }

    /// 80〜200 を 10 刻みで
    fn graphs() -> Vec<LevelGraph> {
        (8..=20)
            .map(|k| {
                let level = k as f64 * 10.0;
                LevelGraph {
                    level,
                    ys: xs().iter().map(|x| Some(level / 10.0 - x)).collect(),
                }
            })
            .collect()
    }

    fn bands(scene: &Scene) -> Vec<(&Vec<Vec<Pt>>, &Paint)> {
        scene
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Path {
                    subpaths,
                    closed: true,
                    fill: Some(fill),
                    stroke: None,
                } => Some((subpaths, fill)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn bands_lie_between_neighboring_levels_and_change_color_every_color_step() {
        let scene = level_graphs(&plot(), &xs(), &graphs(), 20.0, &[]);
        let bands = bands(&scene);
        // 80〜90 から 190〜200 の 12 本。80 の境の下は塗らない
        assert_eq!(bands.len(), 12);
        for (subpaths, _) in &bands {
            for p in subpaths.iter().flatten() {
                // 画面 (sx, sy) → データ (sx/20, 10 − sy/20)。境 80 は x + y = 8
                let (x, y) = (p[0] / 20.0, 10.0 - p[1] / 20.0);
                assert!(x + y >= 8.0 - 1e-9, "80 未満を塗った: {p:?}");
                assert!((0.0..=200.0).contains(&p[0]) && (0.0..=200.0).contains(&p[1]));
            }
        }
        // 色は 20 ごと: 80〜100、100〜120、…、180〜200 の 6 段
        let paints: Vec<&Paint> = bands.iter().map(|(_, p)| *p).collect();
        for pair in paints.chunks(2) {
            assert_eq!(pair[0], pair[1]);
        }
        for w in paints.windows(3).step_by(2) {
            assert_ne!(w[1], w[2]);
        }
        let first = Palette::Seq.band_paint(0, 6);
        let last = Palette::Seq.band_paint(5, 6);
        assert_eq!((paints[0], paints[11]), (&first, &last));
    }

    #[test]
    fn lines_are_thin_ink_inside_the_plot() {
        let scene = level_graphs(&plot(), &xs(), &graphs(), 20.0, &[]);
        let lines: Vec<&Vec<Vec<Pt>>> = scene
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Path {
                    subpaths,
                    closed: false,
                    fill: None,
                    stroke,
                } => {
                    assert_eq!(stroke, &Some(Stroke::new("ink", LINE_WIDTH)));
                    Some(subpaths)
                }
                _ => None,
            })
            .collect();
        // 200 の境は右上の隅に触れるだけ。80〜190 の 12 本は範囲を横切る
        assert!(lines.len() >= 12, "{}", lines.len());
        for p in lines.iter().copied().flatten().flatten() {
            assert!(
                (-1e-9..=200.0 + 1e-9).contains(&p[0]) && (-1e-9..=200.0 + 1e-9).contains(&p[1]),
                "範囲の外: {p:?}"
            );
        }
    }

    #[test]
    fn numbers_sit_on_the_diagonal_preferring_the_color_edges() {
        let scene = level_graphs(&plot(), &xs(), &graphs(), 20.0, &[]);
        let labels: Vec<(f64, f64, &str)> = scene
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Text { x, y, text, .. } => Some((*x, *y, text.as_str())),
                _ => None,
            })
            .collect();
        assert!(!labels.is_empty());
        for (x, y, _) in &labels {
            // 左下 (0, 200) から右上 (200, 0) への対角線: x = 200 − y
            assert!((x + y - 200.0).abs() < 1e-6, "対角線の外: ({x}, {y})");
        }
        let texts: Vec<&str> = labels.iter().map(|l| l.2).collect();
        for t in ["100", "120", "140", "160", "180"] {
            assert!(texts.contains(&t), "{t} が無い: {texts:?}");
        }
    }

    #[test]
    fn numbers_keep_out_of_the_given_rectangles() {
        // 中心の近く（データの (4, 4)〜(6, 6)）を空ける: 120 と 100 の数値はそこに置けない
        let clear = [[[80.0, 80.0], [120.0, 120.0]]];
        let scene = level_graphs(&plot(), &xs(), &graphs(), 20.0, &clear);
        for item in &scene.items {
            if let Item::Text { x, y, text, .. } = item {
                assert!(
                    !((75.0..=125.0).contains(x) && (75.0..=125.0).contains(y)),
                    "{text} が空ける所にある: ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn a_number_kept_off_the_diagonal_moves_along_its_own_line() {
        // 対角線の中心の近くを空ける: 100 の数値は対角線に置けず、100 の線（x + y = 10）の別の場所に出る
        let clear = [[[80.0, 80.0], [120.0, 120.0]]];
        let scene = level_graphs(&plot(), &xs(), &graphs(), 20.0, &clear);
        let hundred: Vec<(f64, f64)> = scene
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Text { x, y, text, .. } if text == "100" => Some((*x, *y)),
                _ => None,
            })
            .collect();
        let [(x, y)] = hundred[..] else {
            panic!("100 の数値が 1 つでない: {hundred:?}");
        };
        // 画面 (sx, sy) → データ (sx/20, 10 − sy/20)
        assert!(
            (x / 20.0 + 10.0 - y / 20.0 - 10.0).abs() < 1e-6,
            "100 の線の外: ({x}, {y})"
        );
        assert!(!((80.0..=120.0).contains(&x) && (80.0..=120.0).contains(&y)));
    }

    #[test]
    fn bands_do_not_repeat_points_where_they_run_along_the_frame() {
        let scene = level_graphs(&plot(), &xs(), &graphs(), 20.0, &[]);
        for (subpaths, _) in bands(&scene) {
            for polygon in subpaths {
                assert!(
                    polygon.windows(2).all(|w| w[0] != w[1]),
                    "同じ点が続く: {polygon:?}"
                );
            }
        }
    }

    #[test]
    fn a_level_without_boundary_fills_up_to_the_top() {
        let mut graphs = graphs()[..3].to_vec();
        // 100 の境は x ≥ 5 で無い（その x ではどの y でも 100 に届かない）
        graphs[2].ys = xs()
            .iter()
            .map(|&x| (x < 5.0).then_some(10.0 - x))
            .collect();
        let scene = level_graphs(&plot(), &xs(), &graphs, 20.0, &[]);
        let bands = bands(&scene);
        assert_eq!(bands.len(), 2);
        // 90〜100 の帯は x ≥ 5 で上の端（画面の y = 0）まで届く
        let top = bands[1]
            .0
            .iter()
            .flatten()
            .filter(|p| p[0] >= 100.0)
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min);
        assert_eq!(top, 0.0);
    }
}
