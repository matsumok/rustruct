//! グリッド → 等値線と帯。純粋関数。
//!
//! セルを中心の点（値は 4 隅の平均）で 4 つの三角形に割り、三角形の中は線形とみなす。
//! 鞍のセル（対角の 2 隅が高く、残りが低い）は中心の値で高い側どうしをつなぐか決める
//! （marching squares の、平均値で曖昧さを解く規則と同じ）。帯と線を同じ三角形から作るので、
//! 帯の境と線がずれない。

use std::collections::HashMap;

use crate::grid::Grid;
use crate::scene::Pt;

/// 1 つの水準の等値線。閉じた線は始点を終点に重ねて返す。端で切れる線は境界に端点を持つ。
#[derive(Debug, Clone, PartialEq)]
pub struct Isoline {
    pub level: f64,
    pub lines: Vec<Vec<Pt>>,
}

/// `lo` 以上 `hi` 未満の帯。最初の帯の `lo` と最後の帯の `hi` は無限大で、帯は面を隙間なく覆う。
/// `polygons` はつながった範囲ごとの外周（反時計回り）と、範囲の中の穴の内周（時計回り）。
/// 塗りの nonzero の規則で穴が抜ける。
#[derive(Debug, Clone, PartialEq)]
pub struct Band {
    pub lo: f64,
    pub hi: f64,
    pub polygons: Vec<Vec<Pt>>,
}

/// `levels` は等値線の水準（昇順）。帯は `levels.len() + 1` 本で、`k` 番目は `levels[k-1]` から `levels[k]`。
pub fn contour(grid: &Grid, levels: &[f64]) -> (Vec<Isoline>, Vec<Band>) {
    let nb = levels.len() + 1;
    let mut bands: Vec<Band> = (0..nb)
        .map(|k| Band {
            lo: if k == 0 {
                f64::NEG_INFINITY
            } else {
                levels[k - 1]
            },
            hi: if k == nb - 1 {
                f64::INFINITY
            } else {
                levels[k]
            },
            polygons: Vec::new(),
        })
        .collect();
    let mut segments: Vec<Vec<(Pt, Pt)>> = vec![Vec::new(); levels.len()];

    let (xs, ys) = (grid.xs(), grid.ys());
    for j in 0..ys.len() - 1 {
        for i in 0..xs.len() - 1 {
            let corners = [
                Vertex::new(xs[i], ys[j], grid.at(i, j)),
                Vertex::new(xs[i + 1], ys[j], grid.at(i + 1, j)),
                Vertex::new(xs[i + 1], ys[j + 1], grid.at(i + 1, j + 1)),
                Vertex::new(xs[i], ys[j + 1], grid.at(i, j + 1)),
            ];
            let center = Vertex::new(
                (xs[i] + xs[i + 1]) / 2.0,
                (ys[j] + ys[j + 1]) / 2.0,
                corners.iter().map(|c| c.v).sum::<f64>() / 4.0,
            );
            // 4 隅が同じ帯ならセルごと 1 つの小片にする（線は通らない）
            let band = band_index(levels, corners[0].v);
            if corners.iter().all(|c| band_index(levels, c.v) == band) {
                bands[band]
                    .polygons
                    .push(corners.iter().map(|c| c.p).collect());
                continue;
            }
            for t in 0..4 {
                let tri = [corners[t], corners[(t + 1) % 4], center];
                for band in bands.iter_mut() {
                    let polygon = clip_band(&tri, band.lo, band.hi);
                    if polygon.len() >= 3 {
                        band.polygons.push(polygon);
                    }
                }
                for (segs, &level) in segments.iter_mut().zip(levels) {
                    if let Some(s) = crossing(&tri, level) {
                        segs.push(s);
                    }
                }
            }
        }
    }

    let isolines = levels
        .iter()
        .zip(segments)
        .map(|(&level, segs)| Isoline {
            level,
            lines: join(segs),
        })
        .collect();
    for band in &mut bands {
        band.polygons = outline(&band.polygons);
    }
    (isolines, bands)
}

#[derive(Debug, Clone, Copy)]
struct Vertex {
    p: Pt,
    v: f64,
}

impl Vertex {
    fn new(x: f64, y: f64, v: f64) -> Self {
        Self { p: [x, y], v }
    }
}

/// 値 `v` が入る帯の番号。`levels[k-1] <= v < levels[k]` で `k`。
fn band_index(levels: &[f64], v: f64) -> usize {
    levels.partition_point(|&l| l <= v)
}

/// 辺 `a`–`b` の上で値が `level` になる点。端点の順を座標で揃え、隣の三角形から見ても
/// 同じ浮動小数になるようにする（線をつなぐときに端点を bit で比べる）。
fn interpolate(a: Vertex, b: Vertex, level: f64) -> Vertex {
    let (a, b) = if a.p <= b.p { (a, b) } else { (b, a) };
    let t = (level - a.v) / (b.v - a.v);
    // 頂点の上に乗る水準は頂点そのものを返す（丸めで隣のセルとずれない）
    let p = if t <= 0.0 {
        a.p
    } else if t >= 1.0 {
        b.p
    } else {
        [
            a.p[0] + t * (b.p[0] - a.p[0]),
            a.p[1] + t * (b.p[1] - a.p[1]),
        ]
    };
    Vertex { p, v: level }
}

/// 三角形のうち `lo <= v < hi` の部分（Sutherland–Hodgman で 2 回切る）。
fn clip_band(tri: &[Vertex; 3], lo: f64, hi: f64) -> Vec<Pt> {
    let above_lo = clip(tri, |v| v >= lo, lo);
    let below_hi = clip(&above_lo, |v| v < hi, hi);
    below_hi.iter().map(|x| x.p).collect()
}

/// `keep(v)` の側を残す。境の値は `level`。
fn clip(poly: &[Vertex], keep: impl Fn(f64) -> bool, level: f64) -> Vec<Vertex> {
    let mut out = Vec::new();
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        let (ka, kb) = (keep(a.v), keep(b.v));
        if ka {
            out.push(a);
        }
        if ka != kb {
            out.push(interpolate(a, b, level));
        }
    }
    out
}

/// 三角形を `level` の線が横切る線分。頂点が `level` ちょうどなら `level` 以上の側とみなす。
fn crossing(tri: &[Vertex; 3], level: f64) -> Option<(Pt, Pt)> {
    let mut ends = Vec::with_capacity(2);
    for k in 0..3 {
        let (a, b) = (tri[k], tri[(k + 1) % 3]);
        if (a.v >= level) != (b.v >= level) {
            ends.push(interpolate(a, b, level).p);
        }
    }
    match ends[..] {
        [p, q] if p != q => Some((p, q)),
        _ => None,
    }
}

/// 1 つの帯の小片（どれも反時計回り）を、つながった範囲ごとの外周にまとめる。隣り合う小片が
/// 共有する辺は向きが逆なので打ち消し合い、残った辺をつなぐと外周は反時計回り、穴の内周は時計回り
/// になる（塗りの nonzero の規則で穴が抜ける）。辺の端点は bit で比べる（[`interpolate`] が隣の
/// 三角形から見ても同じ浮動小数を返す）。要らない点は [`simplify`] で外す。
fn outline(pieces: &[Vec<Pt>]) -> Vec<Vec<Pt>> {
    // 残っている辺（始点、終点）。打ち消された辺は None
    let mut edges: Vec<Option<(Pt, Pt)>> = Vec::new();
    let mut alive: HashMap<(Key, Key), usize> = HashMap::new();
    for piece in pieces {
        for k in 0..piece.len() {
            let (p, q) = (piece[k], piece[(k + 1) % piece.len()]);
            if key(p) == key(q) {
                continue;
            }
            if let Some(i) = alive.remove(&(key(q), key(p))) {
                edges[i] = None;
            } else {
                alive.insert((key(p), key(q)), edges.len());
                edges.push(Some((p, q)));
            }
        }
    }

    let mut from: HashMap<Key, Vec<usize>> = HashMap::new();
    for (i, edge) in edges.iter().enumerate() {
        if let Some((p, _)) = edge {
            from.entry(key(*p)).or_default().push(i);
        }
    }
    let mut used = vec![false; edges.len()];
    let mut rings = Vec::new();
    for start in 0..edges.len() {
        let Some((p, _)) = edges[start] else { continue };
        if used[start] {
            continue;
        }
        let mut ring = vec![p];
        let mut at = start;
        loop {
            used[at] = true;
            let (_, q) = edges[at].expect("残っている辺");
            if key(q) == key(p) {
                break;
            }
            ring.push(q);
            match from
                .get(&key(q))
                .and_then(|out| out.iter().copied().find(|&i| !used[i]))
            {
                Some(i) => at = i,
                None => break,
            }
        }
        rings.extend(simplify(ring));
    }
    rings
}

/// 閉じた折れ線から、要らない点を外す: 前の点と同じ点、行って戻る折り返しの先、前後の点と縦か横に
/// 一直線に並ぶ点（グリッドの線と板の縁の上）。3 点に満たなくなった輪（面積の無い往復）は `None`。
/// 往復は、水準にごく近いグリッドの点で、補間した点とグリッドの点が bit で揃わないときに残る。
fn simplify(ring: Vec<Pt>) -> Option<Vec<Pt>> {
    let redundant = |a: Pt, b: Pt, c: Pt| {
        key(a) == key(c) || (a[0] == b[0] && b[0] == c[0]) || (a[1] == b[1] && b[1] == c[1])
    };
    let mut out: Vec<Pt> = Vec::with_capacity(ring.len());
    for p in ring {
        loop {
            if out.last().is_some_and(|&q| key(q) == key(p)) {
                break;
            }
            let n = out.len();
            if n >= 2 && redundant(out[n - 2], out[n - 1], p) {
                out.pop();
                continue;
            }
            out.push(p);
            break;
        }
    }
    // 始点のまわり（終点 → 始点 → 2 点目）
    while out.len() >= 3 {
        let n = out.len();
        if key(out[n - 1]) == key(out[0]) || redundant(out[n - 2], out[n - 1], out[0]) {
            out.pop();
        } else if redundant(out[n - 1], out[0], out[1]) {
            out.remove(0);
        } else {
            break;
        }
    }
    (out.len() >= 3).then_some(out)
}

/// 点を bit で比べる鍵。
type Key = (u64, u64);

fn key(p: Pt) -> Key {
    (p[0].to_bits(), p[1].to_bits())
}

/// 線分を端点でつないで折れ線にする。閉じた線は始点を終点に重ねて返す。
fn join(segments: Vec<(Pt, Pt)>) -> Vec<Vec<Pt>> {
    let mut at: HashMap<Key, Vec<usize>> = HashMap::new();
    for (k, (p, q)) in segments.iter().enumerate() {
        at.entry(key(*p)).or_default().push(k);
        at.entry(key(*q)).or_default().push(k);
    }
    let mut used = vec![false; segments.len()];
    let mut lines = Vec::new();
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut line = vec![segments[start].0, segments[start].1];
        // 終点から先へ伸ばし、伸びなくなったら向きを変えて始点側へ伸ばす
        for pass in 0..2 {
            loop {
                let end = *line.last().expect("線分 1 本から始める");
                let next = at[&key(end)].iter().copied().find(|&k| !used[k]);
                let Some(k) = next else { break };
                used[k] = true;
                let (p, q) = segments[k];
                line.push(if key(p) == key(end) { q } else { p });
            }
            if pass == 0 {
                line.reverse();
            }
        }
        lines.push(line);
    }
    lines
}

/// 継ぎ目: 既知の関数のグリッドで、線の位置、閉じる線、端で切れる線、鞍点の扱い、帯の面積。
#[cfg(test)]
mod tests {
    use super::*;

    fn square(lo: f64, hi: f64, n: usize, f: impl Fn(f64, f64) -> f64) -> Grid {
        let axis: Vec<f64> = (0..=n)
            .map(|i| lo + (hi - lo) * i as f64 / n as f64)
            .collect();
        Grid::from_fn(axis.clone(), axis, f).unwrap()
    }

    /// 符号つきの面積。反時計回り（外周）は正、時計回り（穴の内周）は負
    fn signed_area(polygon: &[Pt]) -> f64 {
        let n = polygon.len();
        (0..n)
            .map(|k| {
                let (p, q) = (polygon[k], polygon[(k + 1) % n]);
                p[0] * q[1] - q[0] * p[1]
            })
            .sum::<f64>()
            / 2.0
    }

    fn is_closed(line: &[Pt]) -> bool {
        line.len() > 2 && line[0] == line[line.len() - 1]
    }

    fn on_boundary(p: Pt, lo: f64, hi: f64) -> bool {
        [p[0], p[1]]
            .iter()
            .any(|&c| (c - lo).abs() < 1e-12 || (c - hi).abs() < 1e-12)
    }

    #[test]
    fn plane_gives_a_straight_line_cut_at_the_boundary() {
        // z = x。水準 0.5 はグリッドの頂点の上に乗る（退化した場合も 1 本の線になる）
        let grid = square(0.0, 1.0, 10, |x, _| x);
        let (isolines, _) = contour(&grid, &[0.5]);
        assert_eq!(isolines.len(), 1);
        let lines = &isolines[0].lines;
        assert_eq!(lines.len(), 1, "線は 1 本: {lines:?}");
        let line = &lines[0];
        assert!(line.iter().all(|p| (p[0] - 0.5).abs() < 1e-12));
        assert!(!is_closed(line));
        let ys: Vec<f64> = [line[0], line[line.len() - 1]].map(|p| p[1]).into();
        assert!(ys.contains(&0.0) && ys.contains(&1.0), "両端が境界: {ys:?}");

        // 頂点の上に乗らない水準でも同じ
        let (isolines, _) = contour(&grid, &[0.47]);
        let line = &isolines[0].lines[0];
        assert_eq!(isolines[0].lines.len(), 1);
        assert!(line.iter().all(|p| (p[0] - 0.47).abs() < 1e-12));
    }

    #[test]
    fn plane_bands_tile_the_square() {
        let grid = square(0.0, 1.0, 10, |x, _| x);
        let (_, bands) = contour(&grid, &[0.25, 0.5, 0.75]);
        assert_eq!(bands.len(), 4);
        assert_eq!(bands[0].lo, f64::NEG_INFINITY);
        assert_eq!(bands[0].hi, 0.25);
        assert_eq!(bands[3].lo, 0.75);
        assert_eq!(bands[3].hi, f64::INFINITY);
        for (k, band) in bands.iter().enumerate() {
            let total: f64 = band.polygons.iter().map(|p| signed_area(p)).sum();
            assert!((total - 0.25).abs() < 1e-9, "帯 {k} の面積 {total}");
            let (lo, hi) = (0.25 * k as f64, 0.25 * (k + 1) as f64);
            for p in band.polygons.iter().flatten() {
                assert!(
                    p[0] >= lo - 1e-12 && p[0] <= hi + 1e-12,
                    "帯 {k} の点 {p:?}"
                );
            }
        }
    }

    #[test]
    fn cone_gives_a_closed_circle_and_lines_cut_at_the_corners() {
        // z = √(x² + y²)。水準 0.5 は閉じた円、1.2 は四隅で切れる 4 本の弧（隅の半径は √2）
        let grid = square(-1.0, 1.0, 40, |x, y| x.hypot(y));
        let (isolines, _) = contour(&grid, &[0.5, 1.2]);

        let circle = &isolines[0].lines;
        assert_eq!(circle.len(), 1);
        assert!(is_closed(&circle[0]));
        for p in &circle[0] {
            let r = p[0].hypot(p[1]);
            assert!((r - 0.5).abs() < 0.02, "半径 {r}");
        }
        let length: f64 = circle[0]
            .windows(2)
            .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
            .sum();
        assert!(
            (length - std::f64::consts::PI).abs() < 0.05,
            "周長 {length}"
        );

        let arcs = &isolines[1].lines;
        assert_eq!(arcs.len(), 4, "{arcs:?}");
        for arc in arcs {
            assert!(!is_closed(arc));
            assert!(on_boundary(arc[0], -1.0, 1.0) && on_boundary(arc[arc.len() - 1], -1.0, 1.0));
        }
    }

    #[test]
    fn cone_bands_are_one_outline_per_connected_range_with_the_hole_as_an_inner_ring() {
        // z = √(x² + y²)、水準 0.5 と 1.2。帯は中の円板 1 つ、穴のあいた輪 1 つ（四隅で切れる）、四隅の 4 つ
        let grid = square(-1.0, 1.0, 40, |x, y| x.hypot(y));
        let (isolines, bands) = contour(&grid, &[0.5, 1.2]);
        let counts: Vec<usize> = bands.iter().map(|b| b.polygons.len()).collect();
        assert_eq!(counts, [1, 2, 4], "範囲の数");

        // 輪は外周（正の面積）と穴の内周（負の面積）。穴は中の円板と同じ大きさ
        let disk = signed_area(&bands[0].polygons[0]);
        let mut ring: Vec<f64> = bands[1].polygons.iter().map(|p| signed_area(p)).collect();
        ring.sort_by(f64::total_cmp);
        assert!(ring[0] < 0.0 && ring[1] > 0.0, "{ring:?}");
        assert!(
            (ring[0] + disk).abs() < 1e-9,
            "穴 {} と円板 {disk}",
            ring[0]
        );
        assert!(
            (disk - std::f64::consts::PI * 0.25).abs() < 0.01,
            "円板 {disk}"
        );

        // 帯は正方形を隙間なく、重ならずに覆う
        let total: f64 = bands
            .iter()
            .flat_map(|b| &b.polygons)
            .map(|p| signed_area(p))
            .sum();
        assert!((total - 4.0).abs() < 1e-9, "面積の和 {total}");

        // 外周の点は、正方形の縁か、帯の境の等値線の上にだけある（セルや三角形の小片の隅は残らない）
        let on_isoline = |p: Pt, level: f64| {
            isolines
                .iter()
                .filter(|iso| iso.level == level)
                .flat_map(|iso| iso.lines.iter().flatten())
                .any(|q| (q[0] - p[0]).hypot(q[1] - p[1]) < 1e-9)
        };
        for band in &bands {
            for p in band.polygons.iter().flatten() {
                assert!(
                    on_boundary(*p, -1.0, 1.0)
                        || on_isoline(*p, band.lo)
                        || on_isoline(*p, band.hi),
                    "帯 {}–{} の点 {p:?} が縁にも境にも無い",
                    band.lo,
                    band.hi
                );
            }
        }
    }

    #[test]
    fn saddle_function_gives_two_branches() {
        // z = x·y。水準 0.1 は第 1・第 3 象限の双曲線の 2 枝
        let grid = square(-1.0, 1.0, 20, |x, y| x * y);
        let (isolines, _) = contour(&grid, &[-0.1, 0.1]);
        for iso in &isolines {
            assert_eq!(iso.lines.len(), 2, "水準 {}: {:?}", iso.level, iso.lines);
            for line in &iso.lines {
                assert!(!is_closed(line));
                assert!(line.iter().all(|p| (p[0] * p[1] - iso.level).abs() < 0.01));
                let signs: Vec<bool> = line.iter().map(|p| p[0] > 0.0).collect();
                assert!(signs.iter().all(|&s| s == signs[0]), "枝が象限をまたぐ");
            }
        }
    }

    #[test]
    fn saddle_function_bands_split_into_the_quadrants() {
        // z = x·y、水準 ±0.1。負の帯は第 2・第 4 象限、正の帯は第 1・第 3 象限に 2 つずつ、間の帯は十字の 1 つ
        let grid = square(-1.0, 1.0, 20, |x, y| x * y);
        let (_, bands) = contour(&grid, &[-0.1, 0.1]);
        let counts: Vec<usize> = bands.iter().map(|b| b.polygons.len()).collect();
        assert_eq!(counts, [2, 1, 2]);
        // 象限 1 つの xy > 0.1 の面積は ∫[0.1, 1] (1 − 0.1/x) dx = 0.9 − 0.1·ln 10
        let quadrant = 0.9 - 0.1 * 10f64.ln();
        for k in [0, 2] {
            for p in &bands[k].polygons {
                let a = signed_area(p);
                assert!((a - quadrant).abs() < 0.01, "帯 {k} の範囲 {a}");
                let signs: Vec<bool> = p
                    .iter()
                    .filter(|q| q[0] != 0.0)
                    .map(|q| q[0] > 0.0)
                    .collect();
                assert!(signs.iter().all(|&s| s == signs[0]), "範囲が象限をまたぐ");
            }
        }
        let cross = signed_area(&bands[1].polygons[0]);
        assert!(
            (cross - (4.0 - 4.0 * quadrant)).abs() < 0.04,
            "十字 {cross}"
        );
    }

    #[test]
    fn saddle_cell_is_resolved_by_the_center_value() {
        // 対角の 2 隅が高いセル。中心（平均）が水準より低ければ高い隅が孤立し、高ければ低い隅が孤立する
        let cell = |z: [f64; 4]| {
            Grid::new(vec![0.0, 1.0], vec![0.0, 1.0], vec![z[0], z[1], z[3], z[2]]).unwrap()
        };
        let touches = |line: &[Pt], x: f64, y: f64| {
            line.iter().any(|p| (p[0] - x).abs() < 1e-12)
                && line.iter().any(|p| (p[1] - y).abs() < 1e-12)
        };

        let counts = |bands: &[Band]| bands.iter().map(|b| b.polygons.len()).collect::<Vec<_>>();

        // (0,0) と (1,1) が高く、平均 0.45 < 0.5: 高い隅のまわりに 1 本ずつ。高い帯は隅ごとに 2 つに分かれる
        let (iso, bands) = contour(&cell([1.0, 0.0, 0.8, 0.0]), &[0.5]);
        assert_eq!(counts(&bands), [1, 2]);
        assert_eq!(iso[0].lines.len(), 2);
        assert!(
            iso[0].lines.iter().any(|l| touches(l, 0.0, 0.0)),
            "(0,0) を囲む線が無い"
        );
        assert!(
            iso[0].lines.iter().any(|l| touches(l, 1.0, 1.0)),
            "(1,1) を囲む線が無い"
        );

        // 平均 0.6 > 0.5: 低い隅 (1,0) と (0,1) のまわりに 1 本ずつ。低い帯が隅ごとに 2 つに分かれる
        let (iso, bands) = contour(&cell([1.0, 0.2, 1.0, 0.2]), &[0.5]);
        assert_eq!(counts(&bands), [2, 1]);
        assert_eq!(iso[0].lines.len(), 2);
        assert!(
            iso[0].lines.iter().any(|l| touches(l, 1.0, 0.0)),
            "(1,0) を囲む線が無い"
        );
        assert!(
            iso[0].lines.iter().any(|l| touches(l, 0.0, 1.0)),
            "(0,1) を囲む線が無い"
        );
    }

    #[test]
    fn constant_grid_has_no_lines_and_one_band() {
        let grid = square(0.0, 1.0, 3, |_, _| 2.0);
        let (isolines, bands) = contour(&grid, &[1.0, 3.0]);
        assert!(isolines.iter().all(|iso| iso.lines.is_empty()));
        let areas: Vec<f64> = bands
            .iter()
            .map(|b| b.polygons.iter().map(|p| signed_area(p)).sum())
            .collect();
        assert!(
            (areas[1] - 1.0).abs() < 1e-12 && areas[0] == 0.0 && areas[2] == 0.0,
            "{areas:?}"
        );
        // 縁の上で一直線に並ぶグリッドの点は残さず、正方形の 4 隅だけ
        assert_eq!(bands[1].polygons.len(), 1);
        let mut corners = bands[1].polygons[0].clone();
        corners.sort_by(|p, q| p.partial_cmp(q).unwrap());
        assert_eq!(corners, [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
    }
}
