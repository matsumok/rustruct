//! 支持条件の型ごとの解き方の継ぎ目。返すのは無次元のたわみの導関数だけで、M・Q・V・隅反力への換算は入口（`mod.rs`）の
//! 一箇所で行う（ADR-0002）。座標は代表の向きの板の中央原点、x の半辺長 1、y の半辺長 b = Ly/Lx。

use rustruct_types::kirchhoff_plate::{Corner, Edge};

use super::levy::Derivatives;

pub trait Solver {
    /// 板の内部の点。V はここからは求めない（辺の反力は辺に沿う展開の `edge` からだけ）
    fn interior(&self, x: f64, y: f64) -> Derivatives;

    /// 辺の上の点。`along` は辺に沿う無次元座標（x の辺なら y、y の辺なら x）。
    /// その辺に沿う向きの級数で評価する
    fn edge(&self, edge: Edge, along: f64) -> Derivatives;

    /// 角での w_xy。隅反力の元
    fn corner_w_xy(&self, corner: Corner) -> f64;
}
