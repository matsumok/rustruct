//! 支持条件の型ごとの解き方の継ぎ目。返すのは無次元のたわみの導関数だけで、M・Q・V・隅反力への換算は入口（`mod.rs`）の
//! 一箇所で行う（ADR-0002）。座標は代表の向きの板の中央原点、x の半辺長 1、y の半辺長 b = Ly/Lx。
//!
//! 辺の反力だけは例外で、有限要素が節点の反力から直接返せるように `edge_reaction` を分けてある。級数解は既定の
//! 実装（辺の導関数からの換算）をそのまま使う。

use rustruct_types::kirchhoff_plate::{Corner, Edge};

use super::levy::Derivatives;

pub trait Solver {
    /// 板の内部の点。V はここからは求めない（辺の反力は辺に沿う展開の `edge` からだけ）
    fn interior(&self, x: f64, y: f64) -> Derivatives;

    /// 辺の上の点。`along` は辺に沿う無次元座標（x の辺なら y、y の辺なら x）。
    /// その辺に沿う向きの級数で評価する
    fn edge(&self, edge: Edge, along: f64) -> Derivatives;

    /// 辺の反力（無次元、p = 1、D = 1。支える向きが正）。既定は辺の導関数からの換算で、せん断力とねじり
    /// モーメントの変化の和 −(w_xxx + (2 − ν) w_xyy)（x の辺。y の辺は x と y を入れ替える）を、座標の大きい側の
    /// 辺では符号を変えて支える向きにする
    fn edge_reaction(&self, edge: Edge, along: f64, nu: f64) -> f64 {
        let d = self.edge(edge, along);
        match edge {
            Edge::X0 => -(d.w_xxx + (2.0 - nu) * d.w_xyy),
            Edge::XLx => d.w_xxx + (2.0 - nu) * d.w_xyy,
            Edge::Y0 => -(d.w_yyy + (2.0 - nu) * d.w_xxy),
            Edge::YLy => d.w_yyy + (2.0 - nu) * d.w_xxy,
        }
    }

    /// 辺の反力の辺に沿った積分（無次元）。`half_along` は辺の半長（x の辺なら b、y の辺なら 1）。既定は
    /// `edge_reaction` の台形則で、級数解の辺の反力は辺の上で滑らかなので細かい等分で足りる。有限要素は
    /// 節点の反力の和を厳密に返す
    fn edge_reaction_total(&self, edge: Edge, half_along: f64, nu: f64) -> f64 {
        let divisions = 1000;
        let step = 2.0 * half_along / divisions as f64;
        (0..divisions)
            .map(|k| {
                let a = -half_along + step * k as f64;
                (self.edge_reaction(edge, a, nu) + self.edge_reaction(edge, a + step, nu)) / 2.0
                    * step
            })
            .sum()
    }

    /// 角での w_xy。隅反力の元
    fn corner_w_xy(&self, corner: Corner) -> f64;
}
