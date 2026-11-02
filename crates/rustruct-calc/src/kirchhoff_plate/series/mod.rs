//! 参照解。画面からは呼ばない。有限要素の照合テストが使う。
//!
//! Kirchhoff 板を Lévy 型の単一級数で解く、四辺固定（FFFF）と四辺単純支持（SSSS）の解き方。2026-10-22 に
//! 型ごとの級数解で進めると決め、2026-10-23 に有限要素（`finite_element`）に一本化した（ADR-0002）。
//! 級数解は境界条件を厳密に満たし、項数を増やすと決まった値に収束するので、有限要素の精度を測る物差しとして
//! 残す。入口は `solve` で、級数解の無い型には `None` を返す。外に見えるのは項数 [`TERMS`] だけ。
//!
//! 部品: 単一級数の項（`levy`）、双曲線関数の比の形（`hyperbolic`。大きな引数で溢れないための形で、標準の
//! cosh・sinh・tanh は `clippy.toml` で呼べない）、未知の係数の連立を解く部分ピボットの LU 分解（`linear`）。

mod fixed;
mod hyperbolic;
mod levy;
mod linear;
mod simply_supported;

use rustruct_types::kirchhoff_plate::{Corner, Edge};

use super::PlateError;
use super::solver::{Derivatives, EdgePoint, Solver};
use super::support_type::SupportType;

/// 級数の項数。参照解の中の定数で、値は項数を倍にしたときの変化が収まることで決めた
/// （`tests/kirchhoff_plate_series_reference.rs`、ADR-0006 の 2026-10-23 より前の節）
pub const TERMS: usize = 80;

/// 型に級数解があれば、代表の向きの板（y の半辺長 b = Ly'/Lx'、ポアソン比 ν）を項数 `terms` で解く。級数解の
/// 無い型は `Ok(None)`。四辺固定は未知の係数の連立を解き、それが解けなければ `Singular`
pub(super) fn solve(
    support_type: SupportType,
    b: f64,
    nu: f64,
    terms: usize,
) -> Result<Option<Box<dyn Solver>>, PlateError> {
    Ok(match support_type {
        SupportType::FFFF => Some(Box::new(Reference {
            field: fixed::solve(b, terms).ok_or(PlateError::Singular)?,
            b,
            nu,
        })),
        SupportType::SSSS => Some(Box::new(Reference {
            field: simply_supported::solve(b, terms),
            b,
            nu,
        })),
        _ => None,
    })
}

/// 級数解の型ごとの部分。たわみの導関数だけを返す
trait Field {
    fn interior(&self, x: f64, y: f64) -> Derivatives;
    /// 辺の上の点。その辺に沿う向きの級数で評価する
    fn edge(&self, edge: Edge, along: f64) -> Derivatives;
    fn corner_w_xy(&self, corner: Corner) -> f64;
}

/// 級数解の解き方。辺の反力は辺の導関数から換算し、その合計は数値積分で求める。級数解の型（四辺固定と
/// 四辺単純支持）には自由辺が無い
struct Reference<F> {
    field: F,
    /// y の半辺長 b = Ly'/Lx'
    b: f64,
    nu: f64,
}

impl<F: Field> Reference<F> {
    /// 辺の反力。せん断力とねじりモーメントの変化の和 −(w_xxx + (2 − ν) w_xyy)（x の辺。y の辺は x と y を
    /// 入れ替える）を、座標の大きい側の辺では符号を変えて支える向きにする
    fn reaction(&self, edge: Edge, d: Derivatives) -> f64 {
        let nu = self.nu;
        match edge {
            Edge::X0 => -(d.w_xxx + (2.0 - nu) * d.w_xyy),
            Edge::XLx => d.w_xxx + (2.0 - nu) * d.w_xyy,
            Edge::Y0 => -(d.w_yyy + (2.0 - nu) * d.w_xxy),
            Edge::YLy => d.w_yyy + (2.0 - nu) * d.w_xxy,
        }
    }

    fn edge_reaction(&self, edge: Edge, along: f64) -> f64 {
        self.reaction(edge, self.field.edge(edge, along))
    }
}

impl<F: Field> Solver for Reference<F> {
    fn interior(&self, x: f64, y: f64) -> Derivatives {
        self.field.interior(x, y)
    }

    fn edge(&self, edge: Edge, along: f64) -> EdgePoint {
        let derivatives = self.field.edge(edge, along);
        EdgePoint {
            derivatives,
            reaction: self.reaction(edge, derivatives),
        }
    }

    /// 辺の反力の台形則。級数解の辺の反力は辺の上で滑らかなので細かい等分で足りる
    fn edge_reaction_total(&self, edge: Edge) -> f64 {
        let half_along = match edge {
            Edge::X0 | Edge::XLx => self.b,
            Edge::Y0 | Edge::YLy => 1.0,
        };
        let divisions = 1000;
        let step = 2.0 * half_along / divisions as f64;
        (0..divisions)
            .map(|k| {
                let a = -half_along + step * k as f64;
                (self.edge_reaction(edge, a) + self.edge_reaction(edge, a + step)) / 2.0 * step
            })
            .sum()
    }

    fn corner_w_xy(&self, corner: Corner) -> f64 {
        self.field.corner_w_xy(corner)
    }
}
