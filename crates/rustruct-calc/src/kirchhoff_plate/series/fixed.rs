//! 四辺固定（FFFF）の級数解（参照解）。板の中央を原点に、x の半辺長を 1、y の半辺長を b = Ly/Lx とした無次元の板
//! （p = 1、D = 1）を解く。
//!
//! たわみは、四辺単純支持の板の解（荷重の項）に、x = ±1 の辺と y = ±b の辺に未知の曲げモーメントを
//! 受ける同次解を重ねたもの。固定辺で傾きが 0 になる条件を、辺に沿う三角関数で展開して各項について
//! 課すと、2N 元の連立一次方程式になる。
//!
//! 荷重の項は両方向の単一級数で持つ。内部の点は、双曲線関数の減衰が速いほう（その点から遠い辺の
//! 組を双曲線の向きに持つほう）で評価する。辺の上では、その辺に沿う向きの級数のほうを使う（ADR-0002）。

use rustruct_types::kirchhoff_plate::{Corner, Edge};

use super::super::solver::{Derivatives, Solver};
use super::levy::{Frame, Orientation, Series, alternating, load_edge_slope, moment_edge_slope};
use super::linear::Matrix;

pub struct Fixed {
    /// y の半辺長 b = Ly/Lx
    b: f64,
    /// 荷重の項、y に沿う級数（x の双曲線関数）
    load_along_y: Series,
    /// 荷重の項、x に沿う級数（y の双曲線関数）
    load_along_x: Series,
    /// x = ±1 の辺の曲げモーメントの同次解（y に沿う級数）
    moments_on_x_edges: Series,
    /// y = ±b の辺の曲げモーメントの同次解（x に沿う級数）
    moments_on_y_edges: Series,
}

/// 辺の組ごとの級数の枠と向き。x の辺は y に沿って（半辺長 b）x を横切り（半辺長 1）、y の辺はその逆
#[derive(Clone, Copy)]
struct EdgePair {
    frame: Frame,
    orientation: Orientation,
}

fn edge_pairs(b: f64) -> [EdgePair; 2] {
    [
        EdgePair {
            frame: Frame {
                half_along: b,
                half_across: 1.0,
            },
            orientation: Orientation::SAlongY,
        },
        EdgePair {
            frame: Frame {
                half_along: 1.0,
                half_across: b,
            },
            orientation: Orientation::SAlongX,
        },
    ]
}

/// 連立を立てて解く。項数は辺の組ごとに `terms`。解けなければ `None`
pub fn solve(b: f64, terms: usize) -> Option<Fixed> {
    let n = terms;
    let [x_edges, y_edges] = edge_pairs(b);
    let mut a = Matrix::zeros(2 * n);
    let mut rhs = Vec::with_capacity(2 * n);
    // 未知数は x の辺の係数（0..n）、y の辺の係数（n..2n）。行は同じ並びで、各辺の組の固定辺の傾き 0 を
    // その辺に沿う三角関数の項ごとに課す。自分の組の項は対角に、もう片方の組の項は結合の係数で入る
    for (own_offset, own, other) in [(0, x_edges, y_edges), (n, y_edges, x_edges)] {
        let other_offset = n - own_offset;
        for i in 0..n {
            let beta = own.frame.beta(i + 1);
            let k = beta * own.frame.half_across;
            a.set(own_offset + i, own_offset + i, moment_edge_slope(k) / beta);
            rhs.push(
                2.0 / own.frame.half_along * alternating(i + 1) * load_edge_slope(k) / beta.powi(4),
            );
            for j in 0..n {
                let alpha = other.frame.beta(j + 1);
                a.set(
                    own_offset + i,
                    other_offset + j,
                    coupling(alpha, beta, i + 1, j + 1, own.frame.half_along),
                );
            }
        }
    }
    let m = a.solve(rhs)?;
    let (m_x, m_y) = m.split_at(n);
    Some(Fixed {
        b,
        load_along_y: Series::load(x_edges.frame, x_edges.orientation, n),
        load_along_x: Series::load(y_edges.frame, y_edges.orientation, n),
        moments_on_x_edges: Series::edge_moments(x_edges.frame, x_edges.orientation, m_x.to_vec()),
        moments_on_y_edges: Series::edge_moments(y_edges.frame, y_edges.orientation, m_y.to_vec()),
    })
}

/// 片方の辺の組の同次解の傾きを、もう片方の辺に沿って展開したときの係数:
/// 2 α β (−1)^(m+n) / (h (α² + β²)²)。h は展開する辺の半辺長
fn coupling(alpha: f64, beta: f64, n: usize, m: usize, half_along: f64) -> f64 {
    let sum = alpha * alpha + beta * beta;
    2.0 * alpha * beta * alternating(n + m) / (half_along * sum * sum)
}

impl Solver for Fixed {
    fn interior(&self, x: f64, y: f64) -> Derivatives {
        // y に沿う級数の双曲線関数は (1 − |x|)/b の速さで、x に沿う級数のは (b − |y|) の速さで減衰する
        let load = if (1.0 - x.abs()) / self.b >= self.b - y.abs() {
            &self.load_along_y
        } else {
            &self.load_along_x
        };
        load.at(x, y) + self.moments_on_x_edges.at(x, y) + self.moments_on_y_edges.at(x, y)
    }

    fn edge(&self, edge: Edge, along: f64) -> Derivatives {
        let (x, y, load) = match edge {
            Edge::X0 => (-1.0, along, &self.load_along_y),
            Edge::XLx => (1.0, along, &self.load_along_y),
            Edge::Y0 => (along, -self.b, &self.load_along_x),
            Edge::YLy => (along, self.b, &self.load_along_x),
        };
        load.at(x, y) + self.moments_on_x_edges.at(x, y) + self.moments_on_y_edges.at(x, y)
    }

    fn corner_w_xy(&self, _corner: Corner) -> f64 {
        // 固定辺に沿って傾きが 0 なので、辺に沿った微分 w_xy も辺の上で 0。隅反力は厳密に 0
        0.0
    }
}
