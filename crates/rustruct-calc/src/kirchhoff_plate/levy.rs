//! Lévy 型の単一級数の部品。局所座標 (s, t) で書く: s は三角関数の向き（|s| ≤ half_along、両端は単純支持）、
//! t は双曲線関数の向き（|t| ≤ half_across）。型ごとの解き方はこの部品を向きを変えて組み合わせ、無次元の
//! たわみの導関数（p = 1、D = 1）だけを返す（ADR-0002）。
//!
//! s 方向の形（`Wave`）は cos βs（β = (2n − 1)π/(2 half_along)、s = ±half_along で 0）か、1/4 波の
//! sin β(s + half_along)（β = (2n − 1)π/(4 half_along)、s = −half_along で 0、s = +half_along で傾き 0）。
//! t 方向の形（`Kind`）は双曲線関数の比の形（`hyperbolic`）だけで書く。k = β·half_across。

use std::f64::consts::PI;
use std::ops::Add;

use super::hyperbolic::{
    cosh_over_sinh, cosh_ratio, coth, sech2, sinh_over_sinh, sinh_ratio, tanh,
};

/// 級数の局所の枠。級数に沿う向き s の半辺長と、横切る向き t の半辺長
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub half_along: f64,
    pub half_across: f64,
}

impl Frame {
    /// 項 n（1 から）の波数
    pub fn beta(self, wave: Wave, n: usize) -> f64 {
        match wave {
            Wave::Cos => (2 * n - 1) as f64 * PI / (2.0 * self.half_along),
            Wave::Quarter => (2 * n - 1) as f64 * PI / (4.0 * self.half_along),
        }
    }
}

/// 項の s 方向の形 g。どれも g'' = −β² g
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    /// cos βs、β = (2n − 1)π/(2 half_along)。s について対称で、s = ±half_along で 0
    Cos,
    /// sin β(s + half_along)、β = (2n − 1)π/(4 half_along)。s = −half_along で 0、s = +half_along で傾き 0。
    /// 片方の端だけが支えられた向きの展開に使う
    Quarter,
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
    /// t = ±half_across の辺（`Side` で選ぶ）に曲げモーメント m_n·(s の形) を受ける同次解。両方の辺でたわみ 0、
    /// 選んだ辺で w_tt = −m_n·(s の形)、選ばない辺で w_tt = 0。係数 m_n は型ごとの境界条件の連立で決まる
    EdgeMoment(Side),
    /// t = ±half_across の辺（`Side` で選ぶ）がたわみ δ_n·(s の形) を持つ同次解。選んだ辺で w = δ_n·(s の形)、
    /// 選ばない辺で w = 0、両方の辺で w_tt = 0。係数 δ_n は型ごとの境界条件の連立で決まる
    EdgeDeflection(Side),
}

/// 同次解が値を持つ辺
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// t = ±half_across の両方（同じ値。t について対称）
    Both,
    /// t = −half_across だけ
    Lower,
    /// t = +half_across だけ
    Upper,
}

/// 同次解の t 方向の基底。t について対称（u = cosh βt / cosh k、w = sinh βt / cosh k、κ = k·tanh k）か
/// 反対称（u = sinh βt / sinh k、w = cosh βt / sinh k、κ = k·coth k）。どちらも t = k で u = 1、w = κ/k で、
/// βt で微分すると u と w が入れ替わる
#[derive(Clone, Copy)]
struct Basis {
    u: f64,
    w: f64,
    kappa: f64,
}

impl Basis {
    fn even(bt: f64, k: f64) -> Self {
        Self {
            u: cosh_ratio(bt, k),
            w: sinh_ratio(bt, k),
            kappa: k * tanh(k),
        }
    }

    fn odd(bt: f64, k: f64) -> Self {
        Self {
            u: sinh_over_sinh(bt, k),
            w: cosh_over_sinh(bt, k),
            kappa: k * coth(k),
        }
    }

    /// 辺の曲げモーメントの形 f = (κu − βt·w) / (2β²) とその 1〜3 階の t 導関数。
    /// t = k で f = 0、f'' = −1。t = −k で f = 0、f'' は対称なら −1、反対称なら +1
    fn moment(self, beta: f64, bt: f64) -> [f64; 4] {
        let Self { u, w, kappa } = self;
        [
            (kappa * u - bt * w) / (2.0 * beta * beta),
            ((kappa - 1.0) * w - bt * u) / (2.0 * beta),
            ((kappa - 2.0) * u - bt * w) / 2.0,
            beta * ((kappa - 3.0) * w - bt * u) / 2.0,
        ]
    }

    /// 辺のたわみの形 d = (1 + κ/2)u − (βt/2)w とその 1〜3 階の t 導関数。
    /// t = k で d = 1、d'' = 0。t = −k で d は対称なら 1、反対称なら −1、d'' = 0
    fn deflection(self, beta: f64, bt: f64) -> [f64; 4] {
        let Self { u, w, kappa } = self;
        [
            (1.0 + kappa / 2.0) * u - bt / 2.0 * w,
            beta * ((1.0 + kappa) / 2.0 * w - bt / 2.0 * u),
            beta * beta * (kappa / 2.0 * u - bt / 2.0 * w),
            beta * beta * beta * ((kappa - 1.0) / 2.0 * w - bt / 2.0 * u),
        ]
    }
}

/// 片方の辺だけの形を、対称な形と反対称な形の和と差の半分で作る
fn on_side(
    side: Side,
    beta: f64,
    bt: f64,
    k: f64,
    shape: fn(Basis, f64, f64) -> [f64; 4],
) -> [f64; 4] {
    let even = shape(Basis::even(bt, k), beta, bt);
    let sign = match side {
        Side::Both => return even,
        Side::Upper => 1.0,
        Side::Lower => -1.0,
    };
    let odd = shape(Basis::odd(bt, k), beta, bt);
    std::array::from_fn(|i| (even[i] + sign * odd[i]) / 2.0)
}

/// 項の t 方向の形 h とその 1〜3 階の t 導関数。bt = β t、k = β half_across
pub fn profile(kind: Kind, beta: f64, bt: f64, k: f64) -> [f64; 4] {
    match kind {
        // 両方の辺のたわみ −1 の形。特解と打ち消してたわみ 0、h'' = 0
        Kind::Load => Basis::even(bt, k).deflection(beta, bt).map(|v| -v),
        Kind::EdgeMoment(side) => on_side(side, beta, bt, k, Basis::moment),
        Kind::EdgeDeflection(side) => on_side(side, beta, bt, k, Basis::deflection),
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
    pub wave: Wave,
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
                let beta = frame.beta(Wave::Cos, n);
                2.0 / frame.half_along * alternating(n - 1) / beta.powi(5)
            })
            .collect();
        Self {
            frame,
            wave: Wave::Cos,
            kind: Kind::Load,
            orientation,
            coefficients,
        }
    }

    /// 同次解（`Kind::EdgeMoment`・`Kind::EdgeDeflection`）の級数
    pub fn homogeneous(
        frame: Frame,
        wave: Wave,
        kind: Kind,
        orientation: Orientation,
        coefficients: Vec<f64>,
    ) -> Self {
        Self {
            frame,
            wave,
            kind,
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
            let beta = self.frame.beta(self.wave, i + 1);
            let k = beta * self.frame.half_across;
            let [h, h1, h2, h3] = profile(self.kind, beta, beta * t, k);
            // s の形 g とその s 微分 / β
            let (g, g1) = match self.wave {
                Wave::Cos => ((beta * s).cos(), -(beta * s).sin()),
                Wave::Quarter => {
                    let phase = beta * (s + self.frame.half_along);
                    (phase.sin(), phase.cos())
                }
            };
            l.w += coef * h * g;
            l.w_ss -= coef * h * beta * beta * g;
            l.w_tt += coef * h2 * g;
            l.w_st += coef * h1 * beta * g1;
            l.w_sss -= coef * h * beta * beta * beta * g1;
            l.w_sst -= coef * h1 * beta * beta * g;
            l.w_stt += coef * h2 * beta * g1;
            l.w_ttt += coef * h3 * g;
        }
        l
    }
}
