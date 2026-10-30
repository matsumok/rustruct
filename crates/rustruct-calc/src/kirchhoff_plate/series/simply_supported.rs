//! 四辺単純支持（SSSS）の級数解（参照解）。板の中央を原点に、x の半辺長を 1、y の半辺長を b = Ly/Lx とした無次元の板
//! （p = 1、D = 1）を解く。
//!
//! たわみは荷重の項（`levy::Series::load`）だけで閉じる。荷重の項は、級数に沿う向きの両端と、横切る向きの
//! 両端がともに単純支持の板の解なので、四辺単純支持の板そのもの。未知の係数も連立も無い。
//!
//! 荷重の項は両方向の単一級数で持つ。どちらも同じ板の解で、内部の点では双曲線関数の減衰が速いほう（その点から
//! 遠い辺の組を双曲線の向きに持つほう）で評価する。辺の上では、その辺に沿う向きの級数のほうを使う（ADR-0002）。

use rustruct_types::kirchhoff_plate::{Corner, Edge};

use super::super::solver::{Derivatives, Solver};
use super::levy::{Frame, Orientation, Series};

pub struct SimplySupported {
    /// y の半辺長 b = Ly/Lx
    b: f64,
    /// 荷重の項、y に沿う級数（x の双曲線関数）
    load_along_y: Series,
    /// 荷重の項、x に沿う級数（y の双曲線関数）
    load_along_x: Series,
}

/// 級数を組む。項数は向きごとに `terms`
pub fn solve(b: f64, terms: usize) -> SimplySupported {
    SimplySupported {
        b,
        load_along_y: Series::load(
            Frame {
                half_along: b,
                half_across: 1.0,
            },
            Orientation::SAlongY,
            terms,
        ),
        load_along_x: Series::load(
            Frame {
                half_along: 1.0,
                half_across: b,
            },
            Orientation::SAlongX,
            terms,
        ),
    }
}

impl Solver for SimplySupported {
    fn interior(&self, x: f64, y: f64) -> Derivatives {
        // y に沿う級数の双曲線関数は (1 − |x|)/b の速さで、x に沿う級数のは (b − |y|) の速さで減衰する
        if (1.0 - x.abs()) / self.b >= self.b - y.abs() {
            self.load_along_y.at(x, y)
        } else {
            self.load_along_x.at(x, y)
        }
    }

    fn edge(&self, edge: Edge, along: f64) -> Derivatives {
        match edge {
            Edge::X0 => self.load_along_y.at(-1.0, along),
            Edge::XLx => self.load_along_y.at(1.0, along),
            Edge::Y0 => self.load_along_x.at(along, -self.b),
            Edge::YLy => self.load_along_x.at(along, self.b),
        }
    }

    fn corner_w_xy(&self, corner: Corner) -> f64 {
        // 隅はどちらの級数にとっても辺の上。x に沿う級数で評価する。各項は 1/β³ で減り、符号は隅ごとに揃う
        let (x, y) = match corner {
            Corner::X0Y0 => (-1.0, -self.b),
            Corner::XLxY0 => (1.0, -self.b),
            Corner::XLxYLy => (1.0, self.b),
            Corner::X0YLy => (-1.0, self.b),
        };
        self.load_along_x.at(x, y).w_xy
    }
}
