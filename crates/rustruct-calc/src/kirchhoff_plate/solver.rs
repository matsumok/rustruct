//! 解き方の継ぎ目。返すのは無次元のたわみの導関数だけで、M・Q・V・隅反力への換算は入口（`mod.rs`）の
//! 一箇所で行う（ADR-0002）。座標は代表の向きの板の中央原点、x の半辺長 1、y の半辺長 b = Ly/Lx。
//! 実装は有限要素（`finite_element`。画面が使う）と、参照解の級数（`series`。照合テストだけが使う）。
//!
//! 辺の反力だけは例外で、有限要素が節点の反力から直接返せるように辺の点（`EdgePoint`）に含める。級数解は
//! 辺の導関数から換算する（`series`）。

use std::ops::Add;

use rustruct_types::kirchhoff_plate::{Corner, Edge};

/// 板の座標 (x, y) での無次元のたわみとその導関数。1 階は境界条件の組み立てにしか要らないので持たない
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Derivatives {
    pub w: f64,
    pub w_xx: f64,
    pub w_yy: f64,
    pub w_xy: f64,
    pub w_xxx: f64,
    pub w_xxy: f64,
    pub w_xyy: f64,
    pub w_yyy: f64,
}

impl Add for Derivatives {
    type Output = Derivatives;
    fn add(self, o: Derivatives) -> Derivatives {
        Derivatives {
            w: self.w + o.w,
            w_xx: self.w_xx + o.w_xx,
            w_yy: self.w_yy + o.w_yy,
            w_xy: self.w_xy + o.w_xy,
            w_xxx: self.w_xxx + o.w_xxx,
            w_xxy: self.w_xxy + o.w_xxy,
            w_xyy: self.w_xyy + o.w_xyy,
            w_yyy: self.w_yyy + o.w_yyy,
        }
    }
}

/// 辺の上の 1 点。たわみの導関数と、無次元の辺の反力（p = 1、D = 1。支える向きが正）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgePoint {
    pub derivatives: Derivatives,
    pub reaction: f64,
}

pub trait Solver {
    /// 板の内部の点。V はここからは求めない（辺の反力は `edge` からだけ）
    fn interior(&self, x: f64, y: f64) -> Derivatives;

    /// 辺の上の点。`along` は辺に沿う無次元座標（x の辺なら y、y の辺なら x）
    fn edge(&self, edge: Edge, along: f64) -> EdgePoint;

    /// 辺の反力の辺に沿った積分（無次元）
    fn edge_reaction_total(&self, edge: Edge) -> f64;

    /// 隅での w_xy。隅反力の元
    fn corner_w_xy(&self, corner: Corner) -> f64;

    /// 板全体で |w| が最大の点の座標 (x, y)
    fn deflection_maximum(&self) -> (f64, f64);
}

/// 格子 `xs` × `ys`（両端が辺）の点のうち |w| が最大の点を探し、その点に接する格子の目の中で詰めて、|w| が
/// 最大の点の座標を返す。詰め方は、今の点から ±h の範囲に 9 × 9 点を置き（目の外は目の端に寄せる）、|w| が
/// 最大の点に移って h を 1/4 にする、を 12 回。最初の h は接する目の幅で、最後の点の間隔はその 1e-8 の桁
pub fn deflection_maximum(w: impl Fn(f64, f64) -> f64, xs: &[f64], ys: &[f64]) -> (f64, f64) {
    const SIDE: usize = 4;
    const ROUNDS: usize = 12;
    let (nx, ny) = (xs.len() - 1, ys.len() - 1);
    let magnitude = |x: f64, y: f64| w(x, y).abs();
    let (mut best_i, mut best_j, mut best) = (0, 0, f64::NEG_INFINITY);
    for (j, &y) in ys.iter().enumerate() {
        for (i, &x) in xs.iter().enumerate() {
            let m = magnitude(x, y);
            if m > best {
                (best_i, best_j, best) = (i, j, m);
            }
        }
    }
    let (x_lo, x_hi) = (xs[best_i.saturating_sub(1)], xs[(best_i + 1).min(nx)]);
    let (y_lo, y_hi) = (ys[best_j.saturating_sub(1)], ys[(best_j + 1).min(ny)]);
    let (mut x, mut y) = (xs[best_i], ys[best_j]);
    let (mut hx, mut hy) = ((x_hi - x).max(x - x_lo), (y_hi - y).max(y - y_lo));
    for _ in 0..ROUNDS {
        let (cx, cy) = (x, y);
        for a in 0..=2 * SIDE {
            for b in 0..=2 * SIDE {
                let px = (cx + (a as f64 - SIDE as f64) * hx / SIDE as f64).clamp(x_lo, x_hi);
                let py = (cy + (b as f64 - SIDE as f64) * hy / SIDE as f64).clamp(y_lo, y_hi);
                let m = magnitude(px, py);
                if m > best {
                    (x, y, best) = (px, py, m);
                }
            }
        }
        hx /= SIDE as f64;
        hy /= SIDE as f64;
    }
    (x, y)
}
