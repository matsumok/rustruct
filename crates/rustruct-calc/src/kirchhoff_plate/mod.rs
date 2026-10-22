//! Kirchhoff 板の曲げ。言葉は `docs/tools/kirchhoff-plate/GLOSSARY.md`、判断は同じ所の ADR。
//!
//! 公開の入口は [`solve`] 一つ。入力（[`Plate`]）から [`Solution`] を得て、点の応答、等間隔グリッドの
//! 応答、辺に沿った辺の反力の分布、辺の最大値、角ごとの隅反力を取る。公開の座標は角が原点
//! （x は 0 から Lx、y は 0 から Ly）。符号は ADR-0004。
//!
//! 内部では、支持条件の組を 21 種の型と写し方に分け（[`SupportType`]、ADR-0003）、代表の向きの板を
//! 中央原点・x の半辺長 1 の無次元の板として型ごとの解き方（`solver`）で解く。解き方が返すのは無次元の
//! たわみの導関数だけで、曲げモーメント・せん断力・辺の反力・隅反力への換算はこのファイルの一箇所。

mod fixed;
mod hyperbolic;
mod levy;
mod linear;
mod simply_supported;
mod solver;
mod support_type;

use std::fmt;

use rustruct_types::kirchhoff_plate::{Corner, Edge, Supports};
use rustruct_types::quantity::{
    AreaLoad, Force, ForcePerLength, Length, MomentPerLength, PoissonRatio, Stress,
};

use levy::Derivatives;
use solver::Solver;
use support_type::RepresentativeResponse;
pub use support_type::{Mapping, SupportType};

/// 級数の項数。利用者の入力にも Model にも持たない定数で、値は収束のテストで決めた
/// （`docs/tools/kirchhoff-plate/adr/0006`）
pub const TERMS: usize = 80;

/// 1 枚の板の入力。Model が持つ値と同じ（`docs/tools/kirchhoff-plate/adr/0001`）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plate {
    /// x方向の辺長 Lx
    pub lx: Length,
    /// y方向の辺長 Ly
    pub ly: Length,
    /// 板厚 t
    pub thickness: Length,
    /// ヤング係数 E
    pub young_modulus: Stress,
    /// ポアソン比 ν
    pub poisson_ratio: PoissonRatio,
    /// 等分布荷重 p（下向き正）
    pub load: AreaLoad,
    /// 4 辺の支持条件の組
    pub supports: Supports,
}

/// 入力が不正な理由。辺長・板厚・ヤング係数は正の有限値、荷重は有限値（0 と上向きも可）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidInput {
    Lx,
    Ly,
    Thickness,
    YoungModulus,
    Load,
}

impl fmt::Display for InvalidInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Lx => "x方向の辺長が正の有限値でない",
            Self::Ly => "y方向の辺長が正の有限値でない",
            Self::Thickness => "板厚が正の有限値でない",
            Self::YoungModulus => "ヤング係数が正の有限値でない",
            Self::Load => "等分布荷重が有限値でない",
        })
    }
}

/// 解を求められない理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateError {
    InvalidInput(InvalidInput),
    /// 板を支えきれない支持条件の組（四辺自由、単純支持 1 辺と自由 3 辺）
    Unstable(SupportType),
    /// まだ解き方の無い型
    Unsupported(SupportType),
    /// 級数の係数の連立が解けない。支持の足りない組は手前で弾くので、式か項数の取り方の誤り（サイト ADR-0009）
    Singular,
}

impl fmt::Display for PlateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(reason) => write!(f, "{reason}"),
            Self::Unstable(ty) => write!(f, "支持条件の型 {} は不安定", ty.symbol()),
            Self::Unsupported(ty) => write!(f, "支持条件の型 {} は未対応", ty.symbol()),
            Self::Singular => f.write_str("級数の係数の連立が解けない"),
        }
    }
}

impl std::error::Error for PlateError {}

/// 板の上の 1 点の応答。座標は角が原点
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointResponse {
    pub x: Length,
    pub y: Length,
    /// たわみ（下向き正）
    pub w: Length,
    /// 曲げモーメント（下側引張が正）
    pub mx: MomentPerLength,
    pub my: MomentPerLength,
    /// せん断力（座標の大きい側の断面で下向きが正）
    pub qx: ForcePerLength,
    pub qy: ForcePerLength,
}

/// 辺の上の 1 点の応答。`along` は辺の始点（座標 0 の端）からの距離
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeSample {
    pub along: Length,
    /// 辺に直交する方向の曲げモーメント（x の辺なら Mx）
    pub moment: MomentPerLength,
    /// 辺に直交する断面のせん断力（x の辺なら Qx）
    pub shear: ForcePerLength,
    /// 辺の反力（支える向きが正）
    pub reaction: ForcePerLength,
}

/// 絶対値が最大の値と、辺の始点からのその位置
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extreme {
    pub value: ForcePerLength,
    pub along: Length,
}

/// 辺の最大値（GLOSSARY）。せん断力と辺の反力それぞれの絶対値の最大と位置
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeMaximum {
    pub shear: Extreme,
    pub reaction: Extreme,
}

/// 解。入力と、正規化で決まった型と写し方、代表の向きの板の解き方（係数を持つ）。
/// 代表の向きの辺長や曲げ剛性は入力から導けるので持たない
pub struct Solution {
    plate: Plate,
    support_type: SupportType,
    mapping: Mapping,
    solver: Box<dyn Solver>,
}

/// 入力から解を求める。項数は [`TERMS`]
pub fn solve(plate: &Plate) -> Result<Solution, PlateError> {
    solve_with_terms(plate, TERMS)
}

/// 項数を指定して解く。収束のテスト（ADR-0006）のためにあり、画面は [`solve`] を使う
pub fn solve_with_terms(plate: &Plate, terms: usize) -> Result<Solution, PlateError> {
    validate(plate)?;
    let (support_type, mapping) = SupportType::of(plate.supports);
    let (rep_lx, rep_ly) = mapping.representative_lengths(plate.lx.as_mm(), plate.ly.as_mm());
    let b = rep_ly / rep_lx;
    let solver: Box<dyn Solver> = match support_type {
        SupportType::FFFF => Box::new(fixed::solve(b, terms).ok_or(PlateError::Singular)?),
        SupportType::SSSS => Box::new(simply_supported::solve(b, terms)),
        SupportType::RRRR | SupportType::RRRS => return Err(PlateError::Unstable(support_type)),
        SupportType::FFFR
        | SupportType::FFFS
        | SupportType::FFRR
        | SupportType::FFRS
        | SupportType::FFSS
        | SupportType::FRFR
        | SupportType::FRFS
        | SupportType::FRRR
        | SupportType::FRRS
        | SupportType::FRSR
        | SupportType::FRSS
        | SupportType::FSFS
        | SupportType::FSRS
        | SupportType::FSSS
        | SupportType::RRSS
        | SupportType::RSRS
        | SupportType::RSSS => return Err(PlateError::Unsupported(support_type)),
    };
    Ok(Solution {
        plate: *plate,
        support_type,
        mapping,
        solver,
    })
}

fn validate(plate: &Plate) -> Result<(), InvalidInput> {
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if !positive(plate.lx.as_mm()) {
        return Err(InvalidInput::Lx);
    }
    if !positive(plate.ly.as_mm()) {
        return Err(InvalidInput::Ly);
    }
    if !positive(plate.thickness.as_mm()) {
        return Err(InvalidInput::Thickness);
    }
    if !positive(plate.young_modulus.as_n_per_mm2()) {
        return Err(InvalidInput::YoungModulus);
    }
    if !plate.load.as_n_per_mm2().is_finite() {
        return Err(InvalidInput::Load);
    }
    Ok(())
}

impl From<InvalidInput> for PlateError {
    fn from(reason: InvalidInput) -> Self {
        Self::InvalidInput(reason)
    }
}

impl Solution {
    pub fn plate(&self) -> &Plate {
        &self.plate
    }

    pub fn support_type(&self) -> SupportType {
        self.support_type
    }

    /// 板の上の点の応答。板の外（0 ≤ x ≤ Lx、0 ≤ y ≤ Ly を外れる点）は `None`
    pub fn at(&self, x: Length, y: Length) -> Option<PointResponse> {
        let (x_mm, y_mm) = (x.as_mm(), y.as_mm());
        let (lx, ly) = (self.plate.lx.as_mm(), self.plate.ly.as_mm());
        if !(0.0..=lx).contains(&x_mm) || !(0.0..=ly).contains(&y_mm) {
            return None;
        }
        let (rx, ry) = self.mapping.representative_point(x_mm, y_mm, lx, ly);
        let d = self.solver.interior(self.xi(rx), self.eta(ry));
        let r = self.mapping.point_response_back(self.point_response(d));
        Some(PointResponse {
            x,
            y,
            w: Length::mm(r.w),
            mx: MomentPerLength::nmm_per_mm(r.mx),
            my: MomentPerLength::nmm_per_mm(r.my),
            qx: ForcePerLength::n_per_mm(r.qx),
            qy: ForcePerLength::n_per_mm(r.qy),
        })
    }

    /// 等間隔グリッドの応答。x を `nx` 等分、y を `ny` 等分した (nx + 1)(ny + 1) 点を、y の小さい行から、
    /// 行の中は x の小さい順に並べる
    pub fn grid(&self, nx: usize, ny: usize) -> Vec<PointResponse> {
        let (lx, ly) = (self.plate.lx.as_mm(), self.plate.ly.as_mm());
        let mut out = Vec::with_capacity((nx + 1) * (ny + 1));
        for j in 0..=ny {
            let y = ly * j as f64 / ny as f64;
            for i in 0..=nx {
                let x = lx * i as f64 / nx as f64;
                out.push(
                    self.at(Length::mm(x), Length::mm(y))
                        .expect("グリッドは板の上"),
                );
            }
        }
        out
    }

    /// 辺に沿った応答。辺を `divisions` 等分した divisions + 1 点を、始点（座標 0 の端）から順に
    pub fn edge(&self, edge: Edge, divisions: usize) -> Vec<EdgeSample> {
        let (lx, ly) = (self.plate.lx.as_mm(), self.plate.ly.as_mm());
        let length = match edge {
            Edge::X0 | Edge::XLx => ly,
            Edge::Y0 | Edge::YLy => lx,
        };
        (0..=divisions)
            .map(|i| {
                let along = length * i as f64 / divisions as f64;
                let (rep_edge, rep_along) = self
                    .mapping
                    .representative_edge_position(edge, along, lx, ly);
                let nd = match rep_edge {
                    Edge::X0 | Edge::XLx => self.eta(rep_along),
                    Edge::Y0 | Edge::YLy => self.xi(rep_along),
                };
                let d = self.solver.edge(rep_edge, nd);
                let r = self.edge_response(rep_edge, d);
                EdgeSample {
                    along: Length::mm(along),
                    moment: MomentPerLength::nmm_per_mm(r.moment),
                    shear: ForcePerLength::n_per_mm(
                        self.mapping.edge_shear_back(rep_edge, r.shear),
                    ),
                    reaction: ForcePerLength::n_per_mm(r.reaction),
                }
            })
            .collect()
    }

    /// 辺の最大値。`edge` と同じ評価点から、せん断力と辺の反力それぞれの絶対値の最大と位置
    pub fn edge_maximum(&self, edge: Edge, divisions: usize) -> EdgeMaximum {
        let samples = self.edge(edge, divisions);
        let extreme = |pick: fn(&EdgeSample) -> ForcePerLength| {
            let s = samples
                .iter()
                .max_by(|a, b| {
                    pick(a)
                        .as_n_per_mm()
                        .abs()
                        .total_cmp(&pick(b).as_n_per_mm().abs())
                })
                .expect("評価点は 1 つ以上");
            Extreme {
                value: pick(s),
                along: s.along,
            }
        };
        EdgeMaximum {
            shear: extreme(|s| s.shear),
            reaction: extreme(|s| s.reaction),
        }
    }

    /// 隅反力（板を下に押さえる向きが正）。R = 2 D (1 − ν) s_x s_y w_xy。s_x・s_y は角の座標の符号
    /// （原点の角と対角で +1、ほかの 2 つで −1）で、w_xy の符号が角ごとに逆になるのを揃える
    pub fn corner_reaction(&self, corner: Corner) -> Force {
        let rep = self.mapping.corner(corner);
        let w_xy = self.solver.corner_w_xy(rep);
        let orientation = match rep {
            Corner::X0Y0 | Corner::XLxYLy => 1.0,
            Corner::XLxY0 | Corner::X0YLy => -1.0,
        };
        let a = self.half_x();
        Force::n(2.0 * (1.0 - self.nu()) * self.p() * a * a * orientation * w_xy)
    }

    /// 代表の向きの板の辺長 (Lx', Ly') [mm]
    fn representative_lengths(&self) -> (f64, f64) {
        self.mapping
            .representative_lengths(self.plate.lx.as_mm(), self.plate.ly.as_mm())
    }
    /// 代表の向きの板の x の半辺長 a [mm]。無次元化の長さの単位
    fn half_x(&self) -> f64 {
        self.representative_lengths().0 / 2.0
    }
    /// 曲げ剛性 D = E t³ / (12 (1 − ν²)) [N·mm]。内部でしか使わないので型にしない（サイト ADR-0004）
    fn rigidity(&self) -> f64 {
        let t = self.plate.thickness.as_mm();
        let nu = self.nu();
        self.plate.young_modulus.as_n_per_mm2() * t * t * t / (12.0 * (1.0 - nu * nu))
    }
    fn p(&self) -> f64 {
        self.plate.load.as_n_per_mm2()
    }
    fn nu(&self) -> f64 {
        self.plate.poisson_ratio.value()
    }
    /// 代表の向きの板の角原点の x [mm] → 中央原点の無次元の x
    fn xi(&self, x: f64) -> f64 {
        x / self.half_x() - 1.0
    }
    fn eta(&self, y: f64) -> f64 {
        (y - self.representative_lengths().1 / 2.0) / self.half_x()
    }

    /// 無次元の導関数から内部の単位の応答へ。たわみ w = (p a⁴ / D) ŵ、M = −p a² (ŵ_xx + ν ŵ_yy)、
    /// Q = −p a (ŵ_xxx + ŵ_xyy)。a は代表の向きの板の x の半辺長
    fn point_response(&self, d: Derivatives) -> RepresentativeResponse {
        let (a, p, nu) = (self.half_x(), self.p(), self.nu());
        RepresentativeResponse {
            w: p * a.powi(4) / self.rigidity() * d.w,
            mx: -p * a * a * (d.w_xx + nu * d.w_yy),
            my: -p * a * a * (d.w_yy + nu * d.w_xx),
            qx: -p * a * (d.w_xxx + d.w_xyy),
            qy: -p * a * (d.w_yyy + d.w_xxy),
        }
    }

    /// 辺の上の応答（内部の単位）。反力は V = −p a (ŵ_xxx + (2 − ν) ŵ_xyy)（せん断力とねじりモーメントの
    /// 変化の和）を、座標の大きい側の辺では符号を変えて支える向きにする
    fn edge_response(&self, edge: Edge, d: Derivatives) -> EdgeResponse {
        let r = self.point_response(d);
        let (a, p, nu) = (self.half_x(), self.p(), self.nu());
        match edge {
            Edge::X0 | Edge::XLx => {
                let v = -p * a * (d.w_xxx + (2.0 - nu) * d.w_xyy);
                EdgeResponse {
                    moment: r.mx,
                    shear: r.qx,
                    reaction: if edge == Edge::XLx { -v } else { v },
                }
            }
            Edge::Y0 | Edge::YLy => {
                let v = -p * a * (d.w_yyy + (2.0 - nu) * d.w_xxy);
                EdgeResponse {
                    moment: r.my,
                    shear: r.qy,
                    reaction: if edge == Edge::YLy { -v } else { v },
                }
            }
        }
    }
}

/// 代表の向きで求めた辺の上の応答（内部の単位）。辺に直交する方向の曲げモーメント、辺に直交する断面の
/// せん断力、辺の反力
struct EdgeResponse {
    moment: f64,
    shear: f64,
    reaction: f64,
}
