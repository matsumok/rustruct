//! Kirchhoff 板の曲げ。言葉は `docs/tools/kirchhoff-plate/GLOSSARY.md`、判断は同じ所の ADR。
//!
//! 公開の入口は [`solve`] 一つ。入力（[`Plate`]）から [`Solution`] を得て、点の応答、等間隔グリッドの
//! 応答、辺に沿った辺の反力の分布とその合計、辺の最大値、最大たわみ（ADR kirchhoff-plate-0009）、板全体の最大値
//! （ADR kirchhoff-plate-0010）、隅ごとの隅反力、荷重の合計と釣り合いを取る。
//! 隅の近くの値の見せ方（ADR kirchhoff-plate-0008）の規則もここが持ち（[`CORNER_MARGIN`]、[`Solution::edge_maximum`]、
//! [`Solution::near_corner`]）、画面と照合はそれを呼ぶ。公開の座標は隅が原点
//! （x は 0 から Lx、y は 0 から Ly）。符号は ADR kirchhoff-plate-0004。
//!
//! 内部では、支持条件の組を 21 種の型と写し方に分け（[`SupportType`]、ADR kirchhoff-plate-0003）、代表の向きの板を
//! 中央原点・x の半辺長 1 の無次元の板として、長方形の共形要素の有限要素（`finite_element`）で解く（ADR kirchhoff-plate-0002）。
//! 利用者の座標（隅が原点、mm）と解き方の座標の換算は往きも帰りも `frame` の一箇所で、向きと結果の量の写しは
//! 写し方（`support_type` の `Mapping`）が持つ。
//! 解き方（`solver`）が返すのは無次元のたわみの導関数（と辺の反力）だけで、曲げモーメント・せん断力・隅反力への
//! 換算はこのファイルの一箇所。四辺固定と四辺単純支持の級数解（`series`）は参照解で、照合テストだけが
//! [`solve_with_series`] から呼ぶ。

mod band;
mod finite_element;
mod frame;
mod series;
mod solver;
mod support_type;
#[cfg(test)]
mod tests;

use std::fmt;

/// 入力は Kirchhoff 板 Model そのもの（`rustruct-types`）
pub use rustruct_types::kirchhoff_plate::Plate;
use rustruct_types::kirchhoff_plate::{Corner, Edge, Support};
use rustruct_types::quantity::{Force, ForcePerLength, Length, MomentPerLength};

use frame::Frame;
pub use series::TERMS;
use solver::{Derivatives, EdgePoint, Solver, refine};
use support_type::RepresentativeResponse;
pub use support_type::SupportType;

/// 有限要素の短い辺の分割数。長い辺は辺長に合わせて増やす（`finite_element`）。利用者の入力にも Model にも
/// 持たない定数で、値は分割数を倍にしたときの変化が収まることで決めた（ADR kirchhoff-plate-0006）
pub const DIVISIONS: usize = 40;

/// 板全体の最大値（[`Solution::maxima`]）を探す格子の、短い辺の等分の数。長い辺は辺長に合わせて増やし（偶数）、
/// [`MAXIMA_GRID_LIMIT`] で抑える。格子の点から最大の点を選び、接する目の中で詰める（ADR kirchhoff-plate-0010）
const MAXIMA_GRID: usize = 40;

/// 板全体の最大値を探す格子の、長い辺の等分の数の上限
const MAXIMA_GRID_LIMIT: usize = 400;

/// 同じ大きさとみなす幅。最大の値に対する比（ADR kirchhoff-plate-0009、ADR kirchhoff-plate-0010）
const SAME: f64 = 1e-9;

/// 隅の近くとして除く幅。辺長に対する比。自由辺のある組で、分布図の色の幅と、辺のせん断力・辺の反力の
/// 最大値から除く（ADR kirchhoff-plate-0008）。照合で除く幅と同じ（ADR kirchhoff-plate-0002）
pub const CORNER_MARGIN: f64 = 0.05;

/// 辺の上の位置（辺長に対する比）が、両端から [`CORNER_MARGIN`] より端に寄っているか。評価点がちょうど
/// 5 % に来たときに丸めで除かないよう、少しの幅を見る
fn near_end(fraction: f64) -> bool {
    const ROUNDING: f64 = 1e-9;
    fraction < CORNER_MARGIN - ROUNDING || fraction > 1.0 - CORNER_MARGIN + ROUNDING
}

/// 入力が不正な理由。欄ごとに 1 つで、並びは欄の順。辺長・板厚・ヤング係数は正の有限値、荷重は有限値（0 と
/// 上向きも可）。ν は [`PoissonRatio`](rustruct_types::quantity::PoissonRatio) が作るときに守る
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidInput {
    Lx,
    Ly,
    Thickness,
    YoungModulus,
    Load,
}

/// 欄の下に出す文言。欄の名前は含めない（画面が欄に付ける）
impl fmt::Display for InvalidInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Lx | Self::Ly | Self::Thickness | Self::YoungModulus => "0 より大きい値を入れる",
            Self::Load => "数値で入れる",
        })
    }
}

/// 解を求められない理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlateError {
    /// 不正な欄のすべて。欄の順で、空にはならない
    InvalidInput(Vec<InvalidInput>),
    /// 板を支えきれない支持条件の組（四辺自由、単純支持 1 辺と自由 3 辺）
    Unstable(SupportType),
    /// 連立一次方程式が解けない。支持の足りない組は手前で弾くので、有限要素なら拘束の組み立て、参照解の級数なら
    /// 式か項数の取り方の誤り（ADR site-0009）
    Singular,
}

impl fmt::Display for PlateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(reasons) => {
                let reasons: Vec<String> = reasons
                    .iter()
                    .map(|reason| format!("{reason:?}: {reason}"))
                    .collect();
                write!(f, "入力が不正（{}）", reasons.join("、"))
            }
            Self::Unstable(ty) => write!(f, "支持条件の型 {} は不安定", ty.symbol()),
            Self::Singular => f.write_str("連立一次方程式が解けない"),
        }
    }
}

impl std::error::Error for PlateError {}

/// 板の上の 1 点の応答。座標は隅が原点
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
    /// 辺に直交する断面のせん断力（x の辺なら Qx）。符号は [`PointResponse`] の Qx・Qy と同じ板理論の向き
    /// （ADR kirchhoff-plate-0004）で、辺の反力のように支える向きを正にしていない。下向きの荷重を支える辺では、
    /// x = 0・y = 0 の辺で正、x = Lx・y = Ly の辺で負になる
    pub shear: ForcePerLength,
    /// 辺の反力（支える向きが正）
    pub reaction: ForcePerLength,
}

/// 絶対値が最大の値と、辺の始点からのその位置
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extreme<T> {
    pub value: T,
    pub along: Length,
}

/// 辺の最大値（GLOSSARY）。辺に直交する方向の曲げモーメント、せん断力、辺の反力それぞれの絶対値の最大と位置
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeMaximum {
    pub moment: Extreme<MomentPerLength>,
    pub shear: Extreme<ForcePerLength>,
    pub reaction: Extreme<ForcePerLength>,
}

/// たわみの最大（GLOSSARY）。板全体の |w| の最大の点のたわみ（下向き正）と、その位置（隅が原点）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeflectionMaximum {
    pub x: Length,
    pub y: Length,
    pub w: Length,
}

/// 板の上の点の位置。隅が原点
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: Length,
    pub y: Length,
}

/// 板全体の最大値の 1 つ（GLOSSARY「最大値」）。値と、その位置。位置が無いのは板のどこにもその量が無いとき
/// （曲げモーメントはその符号の値が無いとき。荷重が 0 の板など）で、値は 0
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Maximum<T> {
    pub value: T,
    pub position: Option<Position>,
}

/// 板全体の最大値（GLOSSARY、ADR kirchhoff-plate-0010）。符号は [`PointResponse`] と同じ
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Maxima {
    /// たわみ。[`Solution::deflection_maximum`] の値と位置
    pub w: Maximum<Length>,
    /// Mx の下側引張の最大（正の最大）
    pub mx_bottom: Maximum<MomentPerLength>,
    /// Mx の上側引張の最大（負で絶対値の最大）
    pub mx_top: Maximum<MomentPerLength>,
    pub my_bottom: Maximum<MomentPerLength>,
    pub my_top: Maximum<MomentPerLength>,
    /// |Qx| が最大の点の Qx（符号つき）
    pub qx: Maximum<ForcePerLength>,
    pub qy: Maximum<ForcePerLength>,
}

/// 解。入力と、正規化で決まった型、利用者の座標と解き方の座標の換算（写し方を持つ）、代表の向きの板の解き方
/// （係数を持つ）。曲げ剛性は入力から導けるので持たない
pub struct Solution {
    plate: Plate,
    support_type: SupportType,
    frame: Frame,
    solver: Box<dyn Solver>,
}

/// 入力から解を求める。有限要素で、短い辺の分割数は [`DIVISIONS`]。不安定な 2 型を除く 19 型すべてが解ける
pub fn solve(plate: &Plate) -> Result<Solution, PlateError> {
    solve_with_divisions(plate, DIVISIONS)
}

/// 短い辺の分割数を指定して有限要素で解く。分割数を倍にしたときの変化を見るテスト（ADR kirchhoff-plate-0006）のためにあり、
/// 画面は [`solve`] を使う。
///
/// # Panics
///
/// `divisions` が 4 以上の偶数でないとき。分割数は呼び手（テスト）が決める定数で、利用者の入力ではない
pub fn solve_with_divisions(plate: &Plate, divisions: usize) -> Result<Solution, PlateError> {
    let (support_type, frame) = normalize(plate)?;
    let nu = plate.material.poisson_ratio().value();
    let solver = finite_element::solve(frame.b(), nu, support_type.representative(), divisions)
        .ok_or(PlateError::Singular)?;
    Ok(Solution {
        plate: *plate,
        support_type,
        frame,
        solver: Box::new(solver),
    })
}

/// 参照解。画面からは呼ばない。照合テストが、有限要素の解を級数解と比べるのに使う。四辺固定と四辺単純支持
/// だけに級数解があり、ほかの型は `Ok(None)`。`terms` は級数の項数で、参照解の値は [`TERMS`]
pub fn solve_with_series(plate: &Plate, terms: usize) -> Result<Option<Solution>, PlateError> {
    let (support_type, frame) = normalize(plate)?;
    let nu = plate.material.poisson_ratio().value();
    Ok(
        series::solve(support_type, frame.b(), nu, terms)?.map(|solver| Solution {
            plate: *plate,
            support_type,
            frame,
            solver,
        }),
    )
}

/// 入力を確かめ、支持条件の組を型と写し方に分け、写し方と辺長から座標の換算を組む（代表の向きの板の y の
/// 半辺長 b = Ly'/Lx' もこれが持つ）。不安定な型はここで弾く
fn normalize(plate: &Plate) -> Result<(SupportType, Frame), PlateError> {
    validate(plate)?;
    let (support_type, mapping) = SupportType::with_mapping(plate.supports);
    if !support_type.is_stable() {
        return Err(PlateError::Unstable(support_type));
    }
    Ok((
        support_type,
        Frame::new(mapping, plate.lx.as_mm(), plate.ly.as_mm()),
    ))
}

/// 値の規則を欄ごとに見て、不正な欄を欄の順にすべて集める
fn validate(plate: &Plate) -> Result<(), PlateError> {
    let positive = |v: f64| v.is_finite() && v > 0.0;
    let checks = [
        (positive(plate.lx.as_mm()), InvalidInput::Lx),
        (positive(plate.ly.as_mm()), InvalidInput::Ly),
        (positive(plate.thickness.as_mm()), InvalidInput::Thickness),
        (
            positive(plate.material.young_modulus().as_n_per_mm2()),
            InvalidInput::YoungModulus,
        ),
        (plate.load.as_n_per_mm2().is_finite(), InvalidInput::Load),
    ];
    let invalid: Vec<InvalidInput> = checks
        .into_iter()
        .filter(|(valid, _)| !valid)
        .map(|(_, reason)| reason)
        .collect();
    if invalid.is_empty() {
        Ok(())
    } else {
        Err(PlateError::InvalidInput(invalid))
    }
}

impl Solution {
    pub fn plate(&self) -> &Plate {
        &self.plate
    }

    pub fn support_type(&self) -> SupportType {
        self.support_type
    }

    /// 板の上の点の応答。板の外（0 ≤ x ≤ Lx、0 ≤ y ≤ Ly を外れる点）は `None`。
    ///
    /// 呼ぶたびに、その点の曲げモーメントとせん断力の元になる 2 階・3 階の導関数を、点の周りの要素の中央の値から
    /// 回復する。結果は覚えないので、同じ点を何度も呼ぶとそのたびに計算する。[`Self::grid`] と [`Self::edge`] も
    /// 点の数だけこれを繰り返す
    pub fn at(&self, x: Length, y: Length) -> Option<PointResponse> {
        let (x_mm, y_mm) = (x.as_mm(), y.as_mm());
        let (lx, ly) = (self.plate.lx.as_mm(), self.plate.ly.as_mm());
        if !(0.0..=lx).contains(&x_mm) || !(0.0..=ly).contains(&y_mm) {
            return None;
        }
        let (xi, eta) = self.frame.solver_point(x_mm, y_mm);
        let d = self.solver.interior(xi, eta);
        let r = self
            .frame
            .mapping()
            .point_response_back(self.point_response(d));
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

    /// 辺に沿った応答。辺を `divisions` 等分した divisions + 1 点を、始点（座標 0 の端）から順に。
    /// 境界条件で 0 の曲げモーメント（単純支持と自由の辺）は、ちょうど 0 で返す。有限要素はこの条件を弱い形で
    /// しか満たさず、辺の上の値は 0 にならない。自由辺の反力は解き方が厳密に 0 で返す
    pub fn edge(&self, edge: Edge, divisions: usize) -> Vec<EdgeSample> {
        let (lx, ly) = (self.plate.lx.as_mm(), self.plate.ly.as_mm());
        let moment_vanishes = matches!(
            self.plate.supports.at(edge),
            Support::SimplySupported | Support::Free
        );
        let length = match edge {
            Edge::X0 | Edge::XLx => ly,
            Edge::Y0 | Edge::YLy => lx,
        };
        (0..=divisions)
            .map(|i| {
                let along = length * i as f64 / divisions as f64;
                let (rep_edge, rep_along) = self.frame.solver_edge_position(edge, along);
                let r = self.edge_response(rep_edge, self.solver.edge(rep_edge, rep_along));
                EdgeSample {
                    along: Length::mm(along),
                    moment: MomentPerLength::nmm_per_mm(if moment_vanishes {
                        0.0
                    } else {
                        r.moment
                    }),
                    shear: ForcePerLength::n_per_mm(
                        self.frame.mapping().edge_shear_back(rep_edge, r.shear),
                    ),
                    reaction: ForcePerLength::n_per_mm(r.reaction),
                }
            })
            .collect()
    }

    /// 辺の反力の合計 [N]（辺に沿った積分、支える向きが正）。辺の反力の合計 − 隅反力の合計 = 荷重の合計
    /// （ADR kirchhoff-plate-0004）。有限要素は節点の反力の和そのもので、参照解の級数は辺の反力の数値積分
    pub fn total_reaction(&self, edge: Edge) -> Force {
        let a = self.frame.half_x();
        let v = self
            .solver
            .edge_reaction_total(self.frame.mapping().edge(edge));
        Force::n(self.p() * a * a * v)
    }

    /// 荷重の合計 p · Lx · Ly（下向きが正）
    pub fn total_load(&self) -> Force {
        self.plate.load.over(self.plate.lx, self.plate.ly)
    }

    /// 辺の反力の合計 − 隅反力の合計（支える向きが正）。釣り合いでは荷重の合計 [`Self::total_load`] に等しい
    /// （ADR kirchhoff-plate-0004、ADR kirchhoff-plate-0007）
    pub fn net_reaction(&self) -> Force {
        let edges: f64 = Edge::ALL
            .iter()
            .map(|&e| self.total_reaction(e).as_n())
            .sum();
        let corners: f64 = Corner::ALL
            .iter()
            .map(|&c| self.corner_reaction(c).as_n())
            .sum();
        Force::n(edges - corners)
    }

    /// 辺の最大値。`edge` と同じ評価点から、曲げモーメント・せん断力・辺の反力それぞれの絶対値の最大と位置。
    /// 自由辺のある組では、固定辺と自由辺の隅で発散するせん断力と辺の反力を、両端から辺長の
    /// [`CORNER_MARGIN`] を除いた範囲で取る。曲げモーメントは隅まで含める（ADR kirchhoff-plate-0008）。
    ///
    /// 辺を等分する数 `DIVISIONS` は型の引数で、両端を除いても評価点が残るよう 2 以上でなければ
    /// コンパイルが通らない。
    ///
    /// ```no_run
    /// # use rustruct_calc::kirchhoff_plate::{Plate, solve};
    /// # use rustruct_types::kirchhoff_plate::{Edge, PlateMaterial, Support, Supports};
    /// # use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};
    /// # let plate = Plate {
    /// #     lx: Length::mm(4000.0),
    /// #     ly: Length::mm(5000.0),
    /// #     thickness: Length::mm(200.0),
    /// #     material: PlateMaterial::Direct {
    /// #         young_modulus: Stress::n_per_mm2(20_000.0),
    /// #         poisson_ratio: PoissonRatio::new(0.2).unwrap(),
    /// #     },
    /// #     load: AreaLoad::kn_per_m2(10.0),
    /// #     supports: Supports { y_ly: Support::Free, ..Supports::all(Support::Fixed) },
    /// # };
    /// let maximum = solve(&plate).unwrap().edge_maximum::<2>(Edge::X0);
    /// ```
    ///
    /// ```compile_fail
    /// # use rustruct_calc::kirchhoff_plate::{Plate, solve};
    /// # use rustruct_types::kirchhoff_plate::{Edge, PlateMaterial, Support, Supports};
    /// # use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};
    /// # let plate = Plate {
    /// #     lx: Length::mm(4000.0),
    /// #     ly: Length::mm(5000.0),
    /// #     thickness: Length::mm(200.0),
    /// #     material: PlateMaterial::Direct {
    /// #         young_modulus: Stress::n_per_mm2(20_000.0),
    /// #         poisson_ratio: PoissonRatio::new(0.2).unwrap(),
    /// #     },
    /// #     load: AreaLoad::kn_per_m2(10.0),
    /// #     supports: Supports { y_ly: Support::Free, ..Supports::all(Support::Fixed) },
    /// # };
    /// let maximum = solve(&plate).unwrap().edge_maximum::<1>(Edge::X0);
    /// ```
    pub fn edge_maximum<const DIVISIONS: usize>(&self, edge: Edge) -> EdgeMaximum {
        const {
            assert!(
                DIVISIONS >= 2,
                "両端を除いても評価点が残るよう、辺は 2 等分以上"
            )
        };
        let samples = self.edge(edge, DIVISIONS);
        let free = self.plate.supports.has_free_edge();
        let length = samples.last().expect("評価点は 1 つ以上").along.as_mm();
        // 量の型を跨いで比べるので、内部の単位の数で比べる
        let extreme = |magnitude: fn(&EdgeSample) -> f64, skip_ends: bool| {
            samples
                .iter()
                .filter(|s| !skip_ends || !near_end(s.along.as_mm() / length))
                .max_by(|a, b| magnitude(a).abs().total_cmp(&magnitude(b).abs()))
                .expect("両端を除いても評価点は 1 つ以上")
        };
        let moment = extreme(|s| s.moment.as_nmm_per_mm(), false);
        let shear = extreme(|s| s.shear.as_n_per_mm(), free);
        let reaction = extreme(|s| s.reaction.as_n_per_mm(), free);
        EdgeMaximum {
            moment: Extreme {
                value: moment.moment,
                along: moment.along,
            },
            shear: Extreme {
                value: shear.shear,
                along: shear.along,
            },
            reaction: Extreme {
                value: reaction.reaction,
                along: reaction.along,
            },
        }
    }

    /// たわみの最大。有限要素の節点の w から |w| が最大の節点を選び、その節点に接する要素の中を形状関数の w で
    /// 詰めた点（ADR kirchhoff-plate-0009）。同じ大きさの最大が板を辺に平行な中心線で折り返した点にもあるとき（対辺がともに
    /// 自由な板など）は、原点に近い側（y、次に x の小さいほう）を返す
    pub fn deflection_maximum(&self) -> DeflectionMaximum {
        let (xi, eta) = self.solver.deflection_maximum();
        let (x, y) = self.frame.user_point(xi, eta);
        let w = |x: f64, y: f64| {
            self.at(Length::mm(x), Length::mm(y))
                .expect("板の上の点")
                .w
                .as_mm()
        };
        let (x, y) = self.nearer_origin(|x, y| w(x, y).abs(), x, y);
        DeflectionMaximum {
            x: Length::mm(x),
            y: Length::mm(y),
            w: Length::mm(w(x, y)),
        }
    }

    /// 板全体の最大値（ADR kirchhoff-plate-0010）。たわみは [`Self::deflection_maximum`]。曲げモーメントとせん断力は、
    /// 板を等分した格子（短い辺を 40 等分）の点から最大の点を選び、その点に接する目の中を [`Self::at`] の値で詰める
    /// （詰め方は最大たわみと同じ）。値は有限要素の値そのもので、辺の上の値も 0 に直さない（単純支持の辺の
    /// 曲げモーメントは、ちょうど 0 にならない小さな値が出る）。
    ///
    /// - 曲げモーメントは向きごと（下側引張と上側引張）に取る。取った最大は 0 に丸めずに返し、板のどこにもその
    ///   符号の値が無い（ちょうど 0 か逆の符号だけの）ときだけ、値 0・位置無し
    /// - せん断力は |Q| の最大の点の、符号つきの値。自由辺のある組では隅から辺長の [`CORNER_MARGIN`] 以内の点
    ///   （[`Self::near_corner`]）を除く（ADR kirchhoff-plate-0008）。曲げモーメントは隅まで含める
    /// - 同じ大きさの最大が板を辺に平行な中心線で折り返した点にもあるときは、原点に近い側（y、次に x の小さいほう）
    pub fn maxima(&self) -> Maxima {
        let (lx, ly) = (self.plate.lx.as_mm(), self.plate.ly.as_mm());
        let (short, long) = (lx.min(ly), lx.max(ly));
        let long_divisions =
            (2 * (MAXIMA_GRID as f64 * long / short / 2.0).ceil() as usize).min(MAXIMA_GRID_LIMIT);
        let (nx, ny) = if lx <= ly {
            (MAXIMA_GRID, long_divisions)
        } else {
            (long_divisions, MAXIMA_GRID)
        };
        let positions = |length: f64, n: usize| -> Vec<f64> {
            (0..=n).map(|k| length * k as f64 / n as f64).collect()
        };
        let (xs, ys) = (positions(lx, nx), positions(ly, ny));
        let grid: Vec<PointResponse> = ys
            .iter()
            .flat_map(|&y| xs.iter().map(move |&x| (x, y)))
            .map(|(x, y)| self.response(x, y))
            .collect();
        let free = self.plate.supports.has_free_edge();
        let search = Search {
            solution: self,
            xs: &xs,
            ys: &ys,
            grid: &grid,
        };

        let moment = |pick: fn(&PointResponse) -> f64| {
            let found = |sign: f64| {
                search
                    .peak(|r| sign * pick(r))
                    .map(|(position, r)| (MomentPerLength::nmm_per_mm(pick(&r)), position))
            };
            (found(1.0), found(-1.0))
        };
        let shear = |pick: fn(&PointResponse) -> f64| {
            let magnitude = |r: &PointResponse| {
                if free && self.near_corner(r.x, r.y) {
                    f64::NEG_INFINITY
                } else {
                    pick(r).abs()
                }
            };
            search
                .peak(magnitude)
                .map(|(position, r)| (ForcePerLength::n_per_mm(pick(&r)), position))
        };
        let (mx_bottom, mx_top) = moment(|r| r.mx.as_nmm_per_mm());
        let (my_bottom, my_top) = moment(|r| r.my.as_nmm_per_mm());
        let qx = shear(|r| r.qx.as_n_per_mm());
        let qy = shear(|r| r.qy.as_n_per_mm());
        let w = self.deflection_maximum();
        Maxima {
            w: Maximum {
                value: w.w,
                position: (w.w.as_mm() != 0.0).then_some(Position { x: w.x, y: w.y }),
            },
            mx_bottom: maximum(mx_bottom, MomentPerLength::nmm_per_mm(0.0)),
            mx_top: maximum(mx_top, MomentPerLength::nmm_per_mm(0.0)),
            my_bottom: maximum(my_bottom, MomentPerLength::nmm_per_mm(0.0)),
            my_top: maximum(my_top, MomentPerLength::nmm_per_mm(0.0)),
            qx: maximum(qx, ForcePerLength::n_per_mm(0.0)),
            qy: maximum(qy, ForcePerLength::n_per_mm(0.0)),
        }
    }

    /// 点 (x, y) [mm] の応答。板の上の点だけを呼ぶ
    fn response(&self, x: f64, y: f64) -> PointResponse {
        self.at(Length::mm(x), Length::mm(y)).expect("板の上の点")
    }

    /// 点 (x, y) [mm] で `magnitude` が最大のとき、板を辺に平行な中心線で折り返した点（3 つ）にも同じ大きさ
    /// （差が [`SAME`] の幅）の最大があれば、原点に近い側（y、次に x の小さいほう）を返す。節点や格子を探す順と
    /// 写し方（ADR kirchhoff-plate-0003）で、どちらが出るかが組ごとに変わらないようにするため
    fn nearer_origin(&self, magnitude: impl Fn(f64, f64) -> f64, x: f64, y: f64) -> (f64, f64) {
        let (lx, ly) = (self.plate.lx.as_mm(), self.plate.ly.as_mm());
        let found = magnitude(x, y);
        [(x, y), (lx - x, y), (x, ly - y), (lx - x, ly - y)]
            .into_iter()
            .filter(|&(px, py)| (magnitude(px, py) - found).abs() <= SAME * found.abs())
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)))
            .expect("見つけた点そのものは残る")
    }

    /// 隅から辺長の [`CORNER_MARGIN`] 以内の点か（x と y のどちらも、端から辺長の 5 % より端に寄っている）。
    /// 分布図の色の幅（ADR kirchhoff-plate-0008）と照合（ADR kirchhoff-plate-0002）が除く点。自由辺のある組だけ除くかは呼び手が
    /// `Supports::has_free_edge` で決める
    pub fn near_corner(&self, x: Length, y: Length) -> bool {
        near_end(x.as_mm() / self.plate.lx.as_mm()) && near_end(y.as_mm() / self.plate.ly.as_mm())
    }

    /// 隅反力（板を下に押さえる向きが正）。R = 2 D (1 − ν) s_x s_y w_xy。s_x・s_y は隅の座標の符号
    /// （原点の隅と対角で +1、ほかの 2 つで −1）で、w_xy の符号が隅ごとに逆になるのを揃える。
    ///
    /// 固定辺を含む隅（辺の上で傾きが 0 なので w_xy も 0）と、自由辺どうしの隅（支持が無い）では、構造で
    /// ちょうど 0 を返す。ほかの隅（単純支持の辺どうし、単純支持の辺と自由辺）は w_xy から求める
    pub fn corner_reaction(&self, corner: Corner) -> Force {
        let rep = self.frame.mapping().corner(corner);
        let w_xy = self.solver.corner_w_xy(rep);
        let orientation = match rep {
            Corner::X0Y0 | Corner::XLxYLy => 1.0,
            Corner::XLxY0 | Corner::X0YLy => -1.0,
        };
        let a = self.frame.half_x();
        Force::n(2.0 * (1.0 - self.nu()) * self.p() * a * a * orientation * w_xy)
    }

    /// 曲げ剛性 D = E t³ / (12 (1 − ν²)) [N·mm]。内部でしか使わないので型にしない（ADR site-0004）
    fn rigidity(&self) -> f64 {
        let t = self.plate.thickness.as_mm();
        let nu = self.nu();
        self.plate.material.young_modulus().as_n_per_mm2() * t * t * t / (12.0 * (1.0 - nu * nu))
    }
    fn p(&self) -> f64 {
        self.plate.load.as_n_per_mm2()
    }
    fn nu(&self) -> f64 {
        self.plate.material.poisson_ratio().value()
    }
    /// 無次元の導関数から内部の単位の応答へ。たわみ w = (p a⁴ / D) ŵ、M = −p a² (ŵ_xx + ν ŵ_yy)、
    /// Q = −p a (ŵ_xxx + ŵ_xyy)。a は代表の向きの板の x の半辺長
    fn point_response(&self, d: Derivatives) -> RepresentativeResponse {
        let (a, p, nu) = (self.frame.half_x(), self.p(), self.nu());
        RepresentativeResponse {
            w: p * a.powi(4) / self.rigidity() * d.w,
            mx: -p * a * a * (d.w_xx + nu * d.w_yy),
            my: -p * a * a * (d.w_yy + nu * d.w_xx),
            qx: -p * a * (d.w_xxx + d.w_xyy),
            qy: -p * a * (d.w_yyy + d.w_xxy),
        }
    }

    /// 辺の上の応答（内部の単位）。反力は解き方の無次元の辺の反力 v̂（支える向きが正。有限要素では節点の反力
    /// から、級数解では −(ŵ_xxx + (2 − ν) ŵ_xyy)、せん断力とねじりモーメントの変化の和）を V = p a v̂ で
    /// 有次元にする
    fn edge_response(&self, edge: Edge, point: EdgePoint) -> EdgeResponse {
        let r = self.point_response(point.derivatives);
        let reaction = self.p() * self.frame.half_x() * point.reaction;
        match edge {
            Edge::X0 | Edge::XLx => EdgeResponse {
                moment: r.mx,
                shear: r.qx,
                reaction,
            },
            Edge::Y0 | Edge::YLy => EdgeResponse {
                moment: r.my,
                shear: r.qy,
                reaction,
            },
        }
    }
}

/// 板全体の最大値を探す格子と、その点の応答（y の小さい行から、行の中は x の小さい順）
struct Search<'a> {
    solution: &'a Solution,
    xs: &'a [f64],
    ys: &'a [f64],
    grid: &'a [PointResponse],
}

impl Search<'_> {
    /// `objective`（大きいほど最大。除く点は負の無限大）が最大の点の位置と応答。格子の点から最大の点を選び
    /// （同じ値なら先の点）、接する目の中で詰め、折り返した点の同じ大きさの最大から原点に近い側を選ぶ。詰めた
    /// 最大が 0 以下（板のどこにも正の `objective` が無い）なら None。0 に近い小さな値も丸めない
    fn peak(&self, objective: impl Fn(&PointResponse) -> f64) -> Option<(Position, PointResponse)> {
        let nx = self.xs.len();
        let (best, _) = self.grid.iter().map(&objective).enumerate().fold(
            (0, f64::NEG_INFINITY),
            |(bk, bv), (k, v)| {
                if v > bv { (k, v) } else { (bk, bv) }
            },
        );
        let at = |x: f64, y: f64| objective(&self.solution.response(x, y));
        let (x, y) = refine(at, self.xs, self.ys, (best % nx, best / nx));
        if at(x, y) <= 0.0 {
            return None;
        }
        let (x, y) = self.solution.nearer_origin(at, x, y);
        Some((
            Position {
                x: Length::mm(x),
                y: Length::mm(y),
            },
            self.solution.response(x, y),
        ))
    }
}

/// 見つけた最大を [`Maximum`] に。見つからなければ値 0、位置無し
fn maximum<T>(found: Option<(T, Position)>, zero: T) -> Maximum<T> {
    match found {
        Some((value, position)) => Maximum {
            value,
            position: Some(position),
        },
        None => Maximum {
            value: zero,
            position: None,
        },
    }
}

/// 代表の向きで求めた辺の上の応答（内部の単位）。辺に直交する方向の曲げモーメント、辺に直交する断面の
/// せん断力、辺の反力
struct EdgeResponse {
    moment: f64,
    shear: f64,
    reaction: f64,
}
