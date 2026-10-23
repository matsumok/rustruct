//! 三辺固定一辺自由（FFFR）の級数解。代表の向きは x = ±1 と y = −b が固定、y = +b が自由。板の中央を原点に、
//! x の半辺長を 1、y の半辺長を b = Ly/Lx とした無次元の板（p = 1、D = 1）を解く。
//!
//! たわみは、四辺単純支持の板の解（荷重の項）に、次の同次解を重ねたもの。
//!
//! - x = ±1 の辺の曲げモーメント（y に沿う級数）。y の形は 1/4 波の sin α(y + b)（y = −b で 0、y = +b で傾き 0）。
//!   固定辺の曲げモーメントは自由辺との角で 0 にならないので、y = ±b で 0 になる形では展開しない（角で跳びが
//!   でき、自由辺の反力が角に向かって 1/距離 で発散して、展開で打ち消しきれない）
//! - y = −b の辺の曲げモーメント（x に沿う級数）
//! - y = +b の辺のたわみと曲げモーメント（x に沿う級数）
//!
//! 未知数は x の辺が 2N、y の辺が 3N。条件は、x = ±1 の辺の傾き 0（y の 1/4 波で展開、2N）、y = −b の辺の
//! 傾き 0、y = +b の辺の反力 V_y ∝ w_yyy + (2 − ν) w_xxy が 0、同じ辺の曲げモーメント My ∝ w_yy + ν w_xx が 0
//! （x の cos で展開、N ずつ）。x について対称なので x = +1 の辺だけを見る。自由辺の条件に ν が入るので、
//! 無次元の解も ν に依る（ADR-0005）。
//!
//! 別の向きの級数どうしの結合は、展開する三角関数 φ（波数 α）と同次解の形 f（波数 β）について
//! (α² + β²)² ∫ f φ = [φ' (f'' − (α² + 2β²) f) + φ ((α² + 2β²) f' − f''')]（角括弧は両端の差）で閉じた形に書ける。
//! f とその導関数の辺の値は形（`levy::profile`）から取る。
//!
//! 荷重の項は両方向の単一級数で持ち、内部と辺の評価の選び方は四辺固定と同じ（ADR-0002）。

use rustruct_types::kirchhoff_plate::{Corner, Edge};

use super::levy::{
    Derivatives, Frame, Kind, Orientation, Series, Side, Wave, alternating, profile,
};
use super::linear::Matrix;
use super::solver::Solver;

pub struct ThreeFixedOneFree {
    /// y の半辺長 b = Ly/Lx
    b: f64,
    /// 荷重の項、y に沿う級数（x の双曲線関数）
    load_along_y: Series,
    /// 荷重の項、x に沿う級数（y の双曲線関数）
    load_along_x: Series,
    /// x = ±1 の辺の曲げモーメント
    moments_on_x_edges: Series,
    /// y = −b の辺（固定）の曲げモーメント
    moments_on_fixed_y_edge: Series,
    /// y = +b の辺（自由）のたわみ
    deflections_on_free_edge: Series,
    /// y = +b の辺（自由）の曲げモーメント
    moments_on_free_edge: Series,
}

/// x の辺は y に沿って（半辺長 b）x を横切り（半辺長 1）、y の辺はその逆
fn frames(b: f64) -> (Frame, Frame) {
    (
        Frame {
            half_along: b,
            half_across: 1.0,
        },
        Frame {
            half_along: 1.0,
            half_across: b,
        },
    )
}

/// y の辺の 3 つの同次解の形。並びは未知数の並びと同じ
const Y_EDGE_KINDS: [Kind; 3] = [
    Kind::EdgeMoment(Side::Lower),
    Kind::EdgeDeflection(Side::Upper),
    Kind::EdgeMoment(Side::Upper),
];

/// 連立を立てて解く。項数は y の辺の級数が `terms`、x の辺の級数が 2 × `terms`（1/4 波は同じ波数までに
/// 倍の項が要る）。解けなければ `None`
pub fn solve(b: f64, nu: f64, terms: usize) -> Option<ThreeFixedOneFree> {
    solve_with(b, nu, terms, 2 * terms)
}

pub fn solve_with(b: f64, nu: f64, terms: usize, x_terms: usize) -> Option<ThreeFixedOneFree> {
    let n = terms;
    let (x_frame, y_frame) = frames(b);
    let load_along_y = Series::load(x_frame, Orientation::SAlongY, n);
    let load_along_x = Series::load(y_frame, Orientation::SAlongX, n);
    // 未知数の並び: x の辺（0..2n）、y = −b の曲げモーメント、y = +b のたわみ、y = +b の曲げモーメント（n ずつ）。
    // 行の並び: x = +1 の傾き（2n）、y = −b の傾き、y = +b の反力、y = +b の曲げモーメント（n ずつ）
    let y_col = |kind_index: usize, m: usize| x_terms + kind_index * n + m;
    let (slope_row, reaction_row, moment_row) = (x_terms, x_terms + n, x_terms + 2 * n);
    let mut a = Matrix::zeros(x_terms + 3 * n);
    let mut rhs = vec![0.0; x_terms + 3 * n];

    // x = +1 の辺の傾き w_x を φ_j = sin α_j (y + b) で展開した成分（∫ φ² = b）。j は 0 から数え、
    // φ_j(b) = (−1)^j、φ_j'(−b) = α_j
    for j in 0..x_terms {
        let alpha = x_frame.beta(Wave::Quarter, j + 1);
        a.set(
            j,
            j,
            profile(Kind::EdgeMoment(Side::Both), alpha, alpha, alpha)[1],
        );
        // 荷重の項 c_i cos a_i y · h_i(x) の傾きは c_i h_i'(1) cos a_i y で、
        // ∫ cos a_i y φ_j / b = (−1)^(i+j) a_i / (b (a_i² − α_j²))
        rhs[j] = -(0..n)
            .map(|i| {
                let a_i = x_frame.beta(Wave::Cos, i + 1);
                let slope = profile(Kind::Load, a_i, a_i, a_i)[1];
                load_along_y.coefficients[i] * slope * alternating(i + j) * a_i
                    / (b * (a_i * a_i - alpha * alpha))
            })
            .sum::<f64>();
        // y の辺の項 c cos βx f(y) の傾きは c β (−1)^m f(y)（m は 1 から）
        for m in 0..n {
            let beta = y_frame.beta(Wave::Cos, m + 1);
            let k = beta * b;
            let c = alpha * alpha + 2.0 * beta * beta;
            let sum = alpha * alpha + beta * beta;
            for (kind_index, kind) in Y_EDGE_KINDS.into_iter().enumerate() {
                let [f0, _, f2, _] = profile(kind, beta, -k, k);
                let [_, f1, _, f3] = profile(kind, beta, k, k);
                let upper = alternating(j) * (c * f1 - f3);
                let lower = alpha * (f2 - c * f0);
                let integral = (upper - lower) / (sum * sum);
                a.set(
                    j,
                    y_col(kind_index, m),
                    beta * alternating(m + 1) / b * integral,
                );
            }
        }
    }

    // y の辺の条件を cos β_m x で展開した成分（∫ cos² = 1）
    for m in 0..n {
        let beta = y_frame.beta(Wave::Cos, m + 1);
        let k = beta * b;
        let reaction = |h: [f64; 4]| h[3] - (2.0 - nu) * beta * beta * h[1];
        // w_yy + ν w_xx。x の cos の項では w_xx = −β² w
        let moment = |h: [f64; 4]| h[2] - nu * beta * beta * h[0];
        for (kind_index, kind) in Y_EDGE_KINDS.into_iter().enumerate() {
            let lower = profile(kind, beta, -k, k);
            let upper = profile(kind, beta, k, k);
            let col = y_col(kind_index, m);
            a.set(slope_row + m, col, lower[1]);
            a.set(reaction_row + m, col, reaction(upper));
            a.set(moment_row + m, col, moment(upper));
        }
        // 荷重の項は y = ±b でたわみと w_yy が 0（特解と級数の和で）なので、曲げモーメントの行には入らない
        let c = load_along_x.coefficients[m];
        rhs[slope_row + m] = -c * profile(Kind::Load, beta, -k, k)[1];
        rhs[reaction_row + m] = -c * reaction(profile(Kind::Load, beta, k, k));
        // x の辺の項 e φ_j(y) f(x)（f は波数 α の両辺の曲げモーメントの形）。∫ f cos βx = 2β(−1)^m / (α² + β²)²
        // （m は 0 から）、∫ f'' cos βx はその −β² 倍。y = +b で φ' = φ''' = 0 なので反力の行には入らない
        for j in 0..x_terms {
            let alpha = x_frame.beta(Wave::Quarter, j + 1);
            let sum = alpha * alpha + beta * beta;
            let projection = 2.0 * beta * alternating(m) / (sum * sum);
            // y = −b で w_y = e α f(x)
            a.set(slope_row + m, j, alpha * projection);
            // y = +b で w_yy + ν w_xx = e φ_j(b) (−α² f + ν f'')
            a.set(
                moment_row + m,
                j,
                -alternating(j) * (alpha * alpha + nu * beta * beta) * projection,
            );
        }
    }

    let coefficients = a.solve(rhs)?;
    let along_x = |kind_index: usize| {
        Series::homogeneous(
            y_frame,
            Wave::Cos,
            Y_EDGE_KINDS[kind_index],
            Orientation::SAlongX,
            coefficients[y_col(kind_index, 0)..][..n].to_vec(),
        )
    };
    Some(ThreeFixedOneFree {
        b,
        moments_on_x_edges: Series::homogeneous(
            x_frame,
            Wave::Quarter,
            Kind::EdgeMoment(Side::Both),
            Orientation::SAlongY,
            coefficients[..x_terms].to_vec(),
        ),
        moments_on_fixed_y_edge: along_x(0),
        deflections_on_free_edge: along_x(1),
        moments_on_free_edge: along_x(2),
        load_along_y,
        load_along_x,
    })
}

impl ThreeFixedOneFree {
    /// 荷重の項を除く同次解の和
    fn homogeneous(&self, x: f64, y: f64) -> Derivatives {
        self.moments_on_x_edges.at(x, y)
            + self.moments_on_fixed_y_edge.at(x, y)
            + self.deflections_on_free_edge.at(x, y)
            + self.moments_on_free_edge.at(x, y)
    }
}

impl Solver for ThreeFixedOneFree {
    fn interior(&self, x: f64, y: f64) -> Derivatives {
        // y に沿う級数の双曲線関数は (1 − |x|)/b の速さで、x に沿う級数のは (b − |y|) の速さで減衰する
        let load = if (1.0 - x.abs()) / self.b >= self.b - y.abs() {
            &self.load_along_y
        } else {
            &self.load_along_x
        };
        load.at(x, y) + self.homogeneous(x, y)
    }

    fn edge(&self, edge: Edge, along: f64) -> Derivatives {
        let (x, y, load) = match edge {
            Edge::X0 => (-1.0, along, &self.load_along_y),
            Edge::XLx => (1.0, along, &self.load_along_y),
            Edge::Y0 => (along, -self.b, &self.load_along_x),
            Edge::YLy => (along, self.b, &self.load_along_x),
        };
        load.at(x, y) + self.homogeneous(x, y)
    }

    fn corner_w_xy(&self, _corner: Corner) -> f64 {
        // 4 つの角はどれも固定辺を含む。固定辺に沿って傾きが 0 なので、辺に沿った微分 w_xy も辺の上で 0。
        // 隅反力は厳密に 0
        0.0
    }
}

#[cfg(test)]
mod scratch {
    use super::*;

    #[test]
    fn band() {
        for nu in [0.0] {
            for b in [0.25, 0.5, 1.0, 2.0] {
                let v = |s: &ThreeFixedOneFree, y: f64| { let d = s.edge(Edge::XLx, y); d.w_xxx + (2.0 - nu) * d.w_xyy };
                let a = solve(b, nu, 80).unwrap();
                let c = solve(b, nu, 160).unwrap();
                let ys: Vec<f64> = (0..=1997).map(|i| -b + 2.0 * b * i as f64 / 1997.0).collect();
                let scale = v(&c, 0.0).abs();
                // distance from free corner (y = b) in units of Lx (= 2) and in fraction of Ly
                let mut line = format!("b{b} (Ly/Lx={b}) mid V {scale:.3}:");
                for f in [0.05, 0.1, 0.15, 0.2, 0.3, 0.4] {
                    let d = ys.iter().filter(|&&y| (b - y) / (2.0 * b) >= f && (y + b) / (2.0 * b) >= 0.05).map(|&y| (v(&a, y) - v(&c, y)).abs()).fold(0.0, f64::max) / scale;
                    line += &format!(" [>{f} Ly: {d:.1e}]");
                }
                println!("{line}");
            }
        }
    }
}
