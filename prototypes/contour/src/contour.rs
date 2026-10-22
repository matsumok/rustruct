//! 等間隔グリッド → 等値線と帯。charta の継ぎ目と同じ形の、捨てる実装。
//!
//! セルを中心の値（4 隅の平均）で 4 つの三角形に割り、三角形の中は線形とみなす。
//! 鞍のセルは中心の値で決まる（marching squares の平均値による曖昧さの解消と同じ）。
//! 帯と線を同じ三角形から作るので、帯の境目と線が必ず重なる。

use std::collections::HashMap;

pub type Pt = [f64; 2];

/// z は行優先（`z[j * nx + i]`、i が x、j が y）。
pub struct Grid {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub z: Vec<f64>,
}

impl Grid {
    pub fn from_fn(xs: Vec<f64>, ys: Vec<f64>, f: impl Fn(f64, f64) -> f64) -> Grid {
        let mut z = Vec::with_capacity(xs.len() * ys.len());
        for &y in &ys {
            for &x in &xs {
                z.push(f(x, y));
            }
        }
        Grid { xs, ys, z }
    }

    fn at(&self, i: usize, j: usize) -> f64 {
        self.z[j * self.xs.len() + i]
    }

    pub fn range(&self) -> (f64, f64) {
        self.z
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| (lo.min(v), hi.max(v)))
    }
}

pub struct Isoline {
    pub level: f64,
    pub lines: Vec<Vec<Pt>>,
}

pub struct Band {
    pub lo: f64,
    pub hi: f64,
    pub polygons: Vec<Vec<Pt>>,
}

#[derive(Clone, Copy)]
struct V {
    p: Pt,
    v: f64,
}

/// `edges` は帯の境（昇順）。帯は `edges.len() - 1` 本、等値線は内側の境だけ。
pub fn contour(grid: &Grid, edges: &[f64]) -> (Vec<Isoline>, Vec<Band>) {
    let nb = edges.len() - 1;
    let mut bands: Vec<Band> = (0..nb)
        .map(|k| Band { lo: edges[k], hi: edges[k + 1], polygons: vec![] })
        .collect();
    let inner = &edges[1..nb];
    let mut segs: Vec<Vec<(Pt, Pt)>> = vec![vec![]; inner.len()];

    let (nx, ny) = (grid.xs.len(), grid.ys.len());
    for j in 0..ny - 1 {
        for i in 0..nx - 1 {
            let c = [
                V { p: [grid.xs[i], grid.ys[j]], v: grid.at(i, j) },
                V { p: [grid.xs[i + 1], grid.ys[j]], v: grid.at(i + 1, j) },
                V { p: [grid.xs[i + 1], grid.ys[j + 1]], v: grid.at(i + 1, j + 1) },
                V { p: [grid.xs[i], grid.ys[j + 1]], v: grid.at(i, j + 1) },
            ];
            let mid = V {
                p: [(c[0].p[0] + c[1].p[0]) / 2.0, (c[0].p[1] + c[3].p[1]) / 2.0],
                v: (c[0].v + c[1].v + c[2].v + c[3].v) / 4.0,
            };
            let band_of = |v: f64| edges.partition_point(|&e| e <= v).clamp(1, nb) - 1;
            let ks: Vec<usize> = c.iter().chain([&mid]).map(|x| band_of(x.v)).collect();
            if ks.iter().all(|&k| k == ks[0]) {
                bands[ks[0]].polygons.push(c.iter().map(|x| x.p).collect());
                continue;
            }
            for t in 0..4 {
                let tri = [c[t], c[(t + 1) % 4], mid];
                for (k, band) in bands.iter_mut().enumerate() {
                    let lo = if k == 0 { f64::NEG_INFINITY } else { band.lo };
                    let hi = if k == nb - 1 { f64::INFINITY } else { band.hi };
                    let poly = clip(&clip(&tri, |v| v - lo), |v| hi - v);
                    if poly.len() >= 3 {
                        band.polygons.push(poly.iter().map(|x| x.p).collect());
                    }
                }
                for (l, &level) in inner.iter().enumerate() {
                    if let Some(s) = cross(&tri, level) {
                        segs[l].push(s);
                    }
                }
            }
        }
    }
    let isolines = inner
        .iter()
        .zip(segs)
        .map(|(&level, s)| Isoline { level, lines: join(s) })
        .collect();
    (isolines, bands)
}

/// 辺の上の補間点。端点の順を揃えて、隣の三角形と同じ浮動小数になるようにする。
fn lerp(a: V, b: V, f: impl Fn(f64) -> f64) -> V {
    let (a, b) = if (a.p[0], a.p[1]) <= (b.p[0], b.p[1]) { (a, b) } else { (b, a) };
    let (fa, fb) = (f(a.v), f(b.v));
    let t = fa / (fa - fb);
    V {
        p: [a.p[0] + t * (b.p[0] - a.p[0]), a.p[1] + t * (b.p[1] - a.p[1])],
        v: a.v + t * (b.v - a.v),
    }
}

/// f(v) >= 0 の側を残す（Sutherland–Hodgman）。
fn clip(poly: &[V], f: impl Fn(f64) -> f64 + Copy) -> Vec<V> {
    let mut out = vec![];
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        let (ia, ib) = (f(a.v) >= 0.0, f(b.v) >= 0.0);
        if ia {
            out.push(a);
        }
        if ia != ib {
            out.push(lerp(a, b, f));
        }
    }
    out
}

fn cross(tri: &[V; 3], level: f64) -> Option<(Pt, Pt)> {
    let f = |v: f64| v - level;
    let mut ps = vec![];
    for k in 0..3 {
        let (a, b) = (tri[k], tri[(k + 1) % 3]);
        if (f(a.v) >= 0.0) != (f(b.v) >= 0.0) {
            ps.push(lerp(a, b, f).p);
        }
    }
    (ps.len() == 2 && ps[0] != ps[1]).then(|| (ps[0], ps[1]))
}

fn key(p: Pt) -> (u64, u64) {
    (p[0].to_bits(), p[1].to_bits())
}

/// 線分をつないで折れ線にする。閉じた線は始点を終点に重ねて返す。
fn join(segs: Vec<(Pt, Pt)>) -> Vec<Vec<Pt>> {
    let mut at: HashMap<(u64, u64), Vec<usize>> = HashMap::new();
    for (k, s) in segs.iter().enumerate() {
        at.entry(key(s.0)).or_default().push(k);
        at.entry(key(s.1)).or_default().push(k);
    }
    let mut used = vec![false; segs.len()];
    let mut lines = vec![];
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut line = vec![segs[start].0, segs[start].1];
        for dir in 0..2 {
            loop {
                let end = *line.last().unwrap();
                let next = at[&key(end)].iter().copied().find(|&k| !used[k]);
                let Some(k) = next else { break };
                used[k] = true;
                let s = segs[k];
                line.push(if key(s.0) == key(end) { s.1 } else { s.0 });
            }
            if dir == 0 {
                line.reverse();
            }
        }
        lines.push(line);
    }
    lines
}
