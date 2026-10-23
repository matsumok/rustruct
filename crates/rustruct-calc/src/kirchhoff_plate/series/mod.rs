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

use super::PlateError;
use super::solver::Solver;
use super::support_type::SupportType;

/// 級数の項数。参照解の中の定数で、値は項数を倍にしたときの変化が収まることで決めた
/// （`tests/kirchhoff_plate_series_reference.rs`、ADR-0006 の 2026-10-23 より前の節）
pub const TERMS: usize = 80;

/// 型に級数解があれば、代表の向きの板（y の半辺長 b = Ly'/Lx'）を項数 `terms` で解く。級数解の無い型は
/// `Ok(None)`。四辺固定は未知の係数の連立を解き、それが解けなければ `Singular`
pub(super) fn solve(
    support_type: SupportType,
    b: f64,
    terms: usize,
) -> Result<Option<Box<dyn Solver>>, PlateError> {
    Ok(match support_type {
        SupportType::FFFF => Some(Box::new(
            fixed::solve(b, terms).ok_or(PlateError::Singular)?,
        )),
        SupportType::SSSS => Some(Box::new(simply_supported::solve(b, terms))),
        _ => None,
    })
}
