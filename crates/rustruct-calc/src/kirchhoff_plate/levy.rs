//! Lévy 型の単一級数の部品。局所座標 (s, t) で書く: s は三角関数の向き（|s| ≤ half_along、両端は単純支持）、
//! t は双曲線関数の向き（|t| ≤ half_across）。型ごとの解き方はこの部品を向きを変えて組み合わせ、無次元の
//! たわみの導関数（p = 1、D = 1）だけを返す（ADR-0002）。
//!
//! 項 n の波数は β = (2n − 1)π/(2 half_along)。cos βs は s = ±half_along で 0 になる（s についてたわみ 0 の辺）。
//! 双曲線関数は比の形（`hyperbolic`）だけで、k = β·half_across に対する C = cosh βt / cosh k、S = sinh βt / cosh k。

use std::f64::consts::PI;
use std::ops::Add;

use super::hyperbolic::{cosh_ratio, sech2, sinh_ratio, tanh};

/// 級数の局所の枠。級数に沿う向き s の半辺長と、横切る向き t の半辺長
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub half_along: f64,
    pub half_across: f64,
}

impl Frame {
    /// 項 n（1 から）の波数
    pub fn beta(self, n: usize) -> f64 {
        (2 * n - 1) as f64 * PI / (2.0 * self.half_along)
    }
}

/// 局所座標での無次元のたわみとその導関数。1 階は境界条件の組み立てにしか要らないので持たない
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Local {
    pub w: f64,
    pub w_ss: f64,
    pub w_tt: f64,
    pub w_st: f64,
    pub w_sss: f64,
    pub w_sst: f64,
    pub w_stt: f64,
    pub w_ttt: f64,
}

/// 板の座標 (x, y) での無次元のたわみとその導関数
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

/// 局所座標の向き。s が x に沿うか y に沿うか
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// s = x、t = y
    SAlongX,
    /// s = y、t = x
    SAlongY,
}

impl Orientation {
    /// 板の座標の点を局所座標 (s, t) に
    pub fn local_point(self, x: f64, y: f64) -> (f64, f64) {
        match self {
            Self::SAlongX => (x, y),
            Self::SAlongY => (y, x),
        }
    }

    pub fn to_plate(self, l: Local) -> Derivatives {
        match self {
            Self::SAlongX => Derivatives {
                w: l.w,
                w_xx: l.w_ss,
                w_yy: l.w_tt,
                w_xy: l.w_st,
                w_xxx: l.w_sss,
                w_xxy: l.w_sst,
                w_xyy: l.w_stt,
                w_yyy: l.w_ttt,
            },
            Self::SAlongY => Derivatives {
                w: l.w,
                w_xx: l.w_tt,
                w_yy: l.w_ss,
                w_xy: l.w_st,
                w_xxx: l.w_ttt,
                w_xxy: l.w_stt,
                w_xyy: l.w_sst,
                w_yyy: l.w_sss,
            },
        }
    }
}

/// 項の t 方向の形。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 等分布荷重 1 を受け、t = ±ht で単純支持の板の解。各項の係数は荷重の展開で決まり、
    /// 特解 w_p(s) = (h² − s²)(5h² − s²)/24（h は s の半辺長）は閉じた形で足す
    Load,
    /// t = ±half_across の辺に曲げモーメント m_n cos βs を受ける同次解。その辺でたわみ 0、w_tt = −m_n cos βs。
    /// 係数 m_n は型ごとの境界条件の連立で決まる
    EdgeMoment,
}

/// 項の t 方向の形 h とその 1〜3 階の t 導関数。bt = β t、k = β half_across
fn profile(kind: Kind, beta: f64, bt: f64, k: f64) -> [f64; 4] {
    let c = cosh_ratio(bt, k);
    let s = sinh_ratio(bt, k);
    let kt = k * tanh(k);
    match kind {
        Kind::Load => {
            // h = A·C + (βt/2)·S、A = −1 − k·tanh k / 2。t = ±half_across で h = −1（特解と打ち消してたわみ 0）、h'' = 0
            let a = -1.0 - kt / 2.0;
            [
                a * c + bt / 2.0 * s,
                beta * ((a + 0.5) * s + bt / 2.0 * c),
                beta * beta * ((a + 1.0) * c + bt / 2.0 * s),
                beta * beta * beta * ((a + 1.5) * s + bt / 2.0 * c),
            ]
        }
        Kind::EdgeMoment => {
            // f = (k·tanh k·C − βt·S) / (2β²)。t = ±half_across で f = 0、f'' = −1
            [
                (kt * c - bt * s) / (2.0 * beta * beta),
                ((kt - 1.0) * s - bt * c) / (2.0 * beta),
                ((kt - 2.0) * c - bt * s) / 2.0,
                beta * ((kt - 3.0) * s - bt * c) / 2.0,
            ]
        }
    }
}

/// 境界条件を組み立てるときに要る、t = half_across の辺での t 方向の傾きの係数。
/// 荷重の項: ∂h/∂t = −β · load_edge_slope(k)、load_edge_slope = (tanh k − k·sech² k)/2
pub fn load_edge_slope(k: f64) -> f64 {
    (tanh(k) - k * sech2(k)) / 2.0
}
/// 辺の曲げモーメントの項: ∂f/∂t = −moment_edge_slope(k)/β、moment_edge_slope = (tanh k + k·sech² k)/2
pub fn moment_edge_slope(k: f64) -> f64 {
    (tanh(k) + k * sech2(k)) / 2.0
}

/// (−1)^n
pub fn alternating(n: usize) -> f64 {
    if n.is_multiple_of(2) { 1.0 } else { -1.0 }
}

/// 一つの向きの単一級数。
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub frame: Frame,
    pub kind: Kind,
    pub orientation: Orientation,
    /// 項 1 から順の係数（荷重の項は c_n、辺の曲げモーメントの項は m_n）
    pub coefficients: Vec<f64>,
}

impl Series {
    /// 荷重の項。c_n = (2/h)(−1)^(n−1)/β⁵（h は s の半辺長）
    pub fn load(frame: Frame, orientation: Orientation, terms: usize) -> Self {
        let coefficients = (1..=terms)
            .map(|n| {
                let beta = frame.beta(n);
                2.0 / frame.half_along * alternating(n - 1) / beta.powi(5)
            })
            .collect();
        Self {
            frame,
            kind: Kind::Load,
            orientation,
            coefficients,
        }
    }

    pub fn edge_moments(frame: Frame, orientation: Orientation, coefficients: Vec<f64>) -> Self {
        Self {
            frame,
            kind: Kind::EdgeMoment,
            orientation,
            coefficients,
        }
    }

    /// 板の座標の点での導関数
    pub fn at(&self, x: f64, y: f64) -> Derivatives {
        let (s, t) = self.orientation.local_point(x, y);
        self.orientation.to_plate(self.local(s, t))
    }

    fn local(&self, s: f64, t: f64) -> Local {
        let mut l = Local::default();
        if self.kind == Kind::Load {
            let hs = self.frame.half_along;
            l.w = (hs * hs - s * s) * (5.0 * hs * hs - s * s) / 24.0;
            l.w_ss = (s * s - hs * hs) / 2.0;
            l.w_sss = s;
        }
        for (i, &coef) in self.coefficients.iter().enumerate() {
            let beta = self.frame.beta(i + 1);
            let k = beta * self.frame.half_across;
            let [h, h1, h2, h3] = profile(self.kind, beta, beta * t, k);
            let cos = (beta * s).cos();
            let sin = (beta * s).sin();
            l.w += coef * h * cos;
            l.w_ss -= coef * h * beta * beta * cos;
            l.w_tt += coef * h2 * cos;
            l.w_st -= coef * h1 * beta * sin;
            l.w_sss += coef * h * beta * beta * beta * sin;
            l.w_sst -= coef * h1 * beta * beta * cos;
            l.w_stt -= coef * h2 * beta * sin;
            l.w_ttt += coef * h3 * cos;
        }
        l
    }
}
