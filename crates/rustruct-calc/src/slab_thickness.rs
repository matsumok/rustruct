//! 必要スラブ厚さ。RC 造二方向スラブ（四辺固定）の必要厚さを、RC 規準のたわみ略算式と表18-1（周辺固定）の
//! 最小厚さで求める。言葉は `docs/tools/slab-thickness/GLOSSARY.md`、等厚曲線の判断は同じ所の ADR-0001。
//!
//! 公開の入口は 3 つ。条件だけで決まる共有値（[`shared_values`]）、1 つの検討点の評価（[`evaluate`]）、
//! 等厚曲線群（[`iso_curves`]）。3 つとも同じ検査と同じ土台（[`Basis`]）を通るので、画面に出す E・γ₁・wp は
//! 算定に使った値と食い違わない。E・γ₁ は条件のコンクリート（`rustruct_types::concrete::Concrete`）が Fc から導く。
//!
//! 内部の計算は長さ mm、力 N（荷重は N/mm²、単位体積重量は N/mm³）の数で行い、入口と出口で量の型に直す。

use std::fmt;

use rustruct_types::quantity::{AreaLoad, Length, Stress, UnitWeight};
pub use rustruct_types::slab_thickness::{Conditions, CriteriaSet, Criterion, Spans};

/// 入力が不正な理由。条件の欄ごとに分け、スパンは最後。荷重は 0 以上、それ以外は正の有限値。
/// 並びは知らせる順で、条件欄の誤りをスパンの誤りより先に言う
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidInput {
    FinishLoad,
    LiveLoad,
    Amplification,
    DeflectionRatio,
    DeflectionAbsolute,
    /// 有効なクライテリアが 1 つも無い（判定の無い必要厚さは定まらない）
    NoCriteria,
    SpanX,
    SpanY,
}

impl fmt::Display for InvalidInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::FinishLoad => "仕上げ荷重が 0 以上の有限値でない",
            Self::LiveLoad => "積載荷重が 0 以上の有限値でない",
            Self::Amplification => "変形増大係数が正の有限値でない",
            Self::DeflectionRatio => "変形角が正の有限値でない",
            Self::DeflectionAbsolute => "たわみ絶対量が正の有限値でない",
            Self::NoCriteria => "クライテリアが 1 つも無い",
            Self::SpanX => "x方向スパンが正の有限値でない",
            Self::SpanY => "y方向スパンが正の有限値でない",
        })
    }
}

impl std::error::Error for InvalidInput {}

/// 共有値。検討点のスパンに依らず、条件だけで決まる値。すべての検討点に共通する。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedValues {
    /// ヤング係数 E
    pub young_modulus: Stress,
    /// 鉄筋コンクリートの単位体積重量 γ₁（自重に使う）
    pub rc_unit_weight: UnitWeight,
    /// スラブ自重を除く荷重 wp = 仕上げ荷重 + 積載荷重
    pub wp: AreaLoad,
}

/// 共有値。荷重が 0 以上の有限値でなければ求めない。変形増大係数とクライテリアは共有値に効かないので見ない
pub fn shared_values(conditions: &Conditions) -> Result<SharedValues, InvalidInput> {
    let finish = conditions.finish_load.as_n_per_mm2();
    let live = conditions.live_load.as_n_per_mm2();
    if !is_non_negative(finish) {
        return Err(InvalidInput::FinishLoad);
    }
    if !is_non_negative(live) {
        return Err(InvalidInput::LiveLoad);
    }
    let concrete = conditions.concrete;
    Ok(SharedValues {
        young_modulus: concrete.young_modulus(),
        rc_unit_weight: concrete.rc_unit_weight(),
        wp: AreaLoad::n_per_mm2(finish + live),
    })
}

/// 1 つの検討点の評価の結果。たわみの値は t = t_req のときの値。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluation {
    /// 短辺スパン lx = min(Lx, Ly)
    pub short_span: Length,
    /// 辺長比 λ = ly / lx（1 以上）
    pub aspect_ratio: f64,
    /// 必要スラブ厚 t_req
    pub required_thickness: Length,
    /// 全荷重 w = γ₁·t_req + wp
    pub total_load: AreaLoad,
    /// 弾性たわみ δe
    pub elastic_deflection: Length,
    /// 増大後たわみ = 変形増大係数 × δe
    pub amplified_deflection: Length,
    /// 許容たわみ。有効なたわみに効く条件の上限の最小。たわみに効く条件が 1 つも無ければ `None`
    pub allowable_deflection: Option<Length>,
    /// 決定条件（必要スラブ厚を与えた条件）
    pub governing: Criterion,
}

/// 1 つの検討点の評価。有効なクライテリアをすべて満たす最小の厚さと、その厚さでのたわみ。
/// 増大後たわみが許容たわみとちょうど等しくなるのは、たわみに効く条件が決定条件のときだけ
/// （表18-1 が決めると、たわみは許容に届かない）
pub fn evaluate(conditions: &Conditions, spans: Spans) -> Result<Evaluation, InvalidInput> {
    // 条件を先に見る。条件欄の誤りをスパンの誤りより先に知らせる
    let (basis, active) = checked_basis(conditions)?;
    let (x, y) = (spans.x.as_mm(), spans.y.as_mm());
    if !is_positive(x) {
        return Err(InvalidInput::SpanX);
    }
    if !is_positive(y) {
        return Err(InvalidInput::SpanY);
    }

    let lx = x.min(y);
    let lambda = x.max(y) / lx;
    let (t_req, governing) = governing_thickness(&active, &basis, lx, lambda);

    let w = basis.rc_unit_weight * t_req + basis.wp;
    let delta_e = shape(lambda) * w * lx.powi(4) / (32.0 * basis.young_modulus * t_req.powi(3));

    Ok(Evaluation {
        short_span: Length::mm(lx),
        aspect_ratio: lambda,
        required_thickness: Length::mm(t_req),
        total_load: AreaLoad::n_per_mm2(w),
        elastic_deflection: Length::mm(delta_e),
        amplified_deflection: Length::mm(basis.amplification * delta_e),
        allowable_deflection: allowable_deflection(&active, lx).map(Length::mm),
        governing,
    })
}

/// 一本の等厚曲線。スラブ厚 `thickness` でクライテリア集合をちょうど満たす (Lx, Ly) の軌跡。
/// `ys` は与えた x の並びに対応し、`None` はその x に境界が無い（どんな y でもこの厚さで足りる）こと
#[derive(Debug, Clone, PartialEq)]
pub struct IsoCurve {
    pub thickness: Length,
    pub ys: Vec<Option<Length>>,
}

/// 等厚曲線群。x の並び `xs` の上で、厚さごとの等厚曲線。曲線は y = x に対して対称。
///
/// グリッドの各点で必要厚さを解かず、厚さを固定して曲線を直接引く（ADR-0001）。上枝（y ≥ x）は
/// 変形角・たわみ絶対量・表18-1 とも閉形式。下枝（y < x）は、たわみ絶対量は閉形式、変形角は Newton 法、
/// 表18-1 は二分法で引く
pub fn iso_curves(
    conditions: &Conditions,
    xs: &[Length],
    thicknesses: &[Length],
) -> Result<Vec<IsoCurve>, InvalidInput> {
    let (basis, active) = checked_basis(conditions)?;
    Ok(thicknesses
        .iter()
        .map(|&t| IsoCurve {
            thickness: t,
            ys: xs
                .iter()
                .map(|&x| combined_curve_y(&active, &basis, t.as_mm(), x.as_mm()).map(Length::mm))
                .collect(),
        })
        .collect())
}

/// 条件から決まり、スパンに依らない算定の土台。[`SharedValues`] と同じ値を、算定式の単位の数で持つ
#[derive(Debug, Clone, Copy)]
struct Basis {
    /// ヤング係数 E [N/mm²]
    young_modulus: f64,
    /// 鉄筋コンクリートの単位体積重量 γ₁ [N/mm³]
    rc_unit_weight: f64,
    /// スラブ自重を除く荷重 wp [N/mm²]
    wp: f64,
    /// 変形増大係数 K
    amplification: f64,
}

/// 条件を検査し、土台と有効なクライテリアをそろえる。評価も等厚曲線群もここを通る
fn checked_basis(conditions: &Conditions) -> Result<(Basis, Vec<ActiveCriterion>), InvalidInput> {
    let shared = shared_values(conditions)?;
    if !is_positive(conditions.amplification) {
        return Err(InvalidInput::Amplification);
    }
    let active = active_criteria(&conditions.criteria)?;
    let basis = Basis {
        young_modulus: shared.young_modulus.as_n_per_mm2(),
        rc_unit_weight: shared.rc_unit_weight.as_kn_per_m3() * 1e-6,
        wp: shared.wp.as_n_per_mm2(),
        amplification: conditions.amplification,
    };
    Ok((basis, active))
}

/// 有効なクライテリアを集合の並び（[`Criterion`] の順）で並べる。並びは決定条件が同値のときの優先順でもある
fn active_criteria(criteria: &CriteriaSet) -> Result<Vec<ActiveCriterion>, InvalidInput> {
    let mut active = Vec::new();
    if let Some(ratio) = criteria.deflection_ratio {
        if !is_positive(ratio) {
            return Err(InvalidInput::DeflectionRatio);
        }
        active.push(ActiveCriterion::DeflectionRatio(ratio));
    }
    if let Some(limit) = criteria.deflection_absolute {
        if !is_positive(limit.as_mm()) {
            return Err(InvalidInput::DeflectionAbsolute);
        }
        active.push(ActiveCriterion::DeflectionAbsolute(limit.as_mm()));
    }
    if criteria.min_thickness {
        active.push(ActiveCriterion::MinThickness);
    }
    if active.is_empty() {
        return Err(InvalidInput::NoCriteria);
    }
    Ok(active)
}

fn is_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn is_non_negative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

/// 有効なクライテリア 1 つ。必要厚さも等厚曲線もここから引くので、条件の種類を足すときは
/// この enum の枝を埋めれば、合成（max・min）の側は触らずに済む
#[derive(Debug, Clone, Copy, PartialEq)]
enum ActiveCriterion {
    /// 変形角（増大後たわみ / lx の上限）
    DeflectionRatio(f64),
    /// たわみ絶対量 [mm]
    DeflectionAbsolute(f64),
    /// 表18-1 最小厚さ
    MinThickness,
}

impl ActiveCriterion {
    fn kind(self) -> Criterion {
        match self {
            Self::DeflectionRatio(_) => Criterion::DeflectionRatio,
            Self::DeflectionAbsolute(_) => Criterion::DeflectionAbsolute,
            Self::MinThickness => Criterion::MinThickness,
        }
    }

    /// この条件が課す増大後たわみの上限 [mm]。たわみに効かない条件（表18-1）は `None`
    fn allowable_deflection(self, lx: f64) -> Option<f64> {
        match self {
            Self::DeflectionRatio(ratio) => Some(lx * ratio),
            Self::DeflectionAbsolute(limit) => Some(limit),
            Self::MinThickness => None,
        }
    }

    /// 短辺 lx [mm]・辺長比 λ でこの条件が求める厚さ [mm]
    fn required_thickness(self, basis: &Basis, lx: f64, lambda: f64) -> f64 {
        match self {
            Self::DeflectionRatio(ratio) => deflection_thickness(basis, lx, lambda, lx * ratio),
            Self::DeflectionAbsolute(limit) => deflection_thickness(basis, lx, lambda, limit),
            Self::MinThickness => table18_thickness(basis, lx, lambda),
        }
    }

    /// 厚さ t [mm] を固定したときのこの条件の等厚曲線の、x [mm] での y [mm]。境界が無ければ `None`
    fn curve_y(self, basis: &Basis, t: f64, x: f64) -> Option<f64> {
        match self {
            Self::DeflectionRatio(ratio) => ratio_curve_y(curve_constant(basis, t, ratio), x),
            Self::DeflectionAbsolute(limit) => absolute_curve_y(curve_constant(basis, t, limit), x),
            Self::MinThickness => table18_curve_y(basis, t, x),
        }
    }
}

/// 有効な条件ごとの必要厚さの最大（すべてを同時に満たす最小の厚さ）と、その決定条件。
/// 同値のときは集合の並びで先の条件を残す
fn governing_thickness(
    active: &[ActiveCriterion],
    basis: &Basis,
    lx: f64,
    lambda: f64,
) -> (f64, Criterion) {
    active
        .iter()
        .map(|c| (c.required_thickness(basis, lx, lambda), c.kind()))
        .reduce(|governing, other| {
            if other.0 > governing.0 {
                other
            } else {
                governing
            }
        })
        .expect("有効な条件が空の集合は active_criteria が弾いている")
}

/// 各条件の等厚曲線の点ごとの最小（すべてを満たす領域の境界）。境界を持つ条件が無ければ `None`
fn combined_curve_y(active: &[ActiveCriterion], basis: &Basis, t: f64, x: f64) -> Option<f64> {
    active
        .iter()
        .filter_map(|c| c.curve_y(basis, t, x))
        .reduce(f64::min)
}

/// 許容たわみ [mm]。たわみに効く条件の上限の最小。表18-1 だけの集合では上限を課す条件が無く、
/// +∞ で表すと「とても大きい許容値」に見えるので、値が無いこと（`None`）として持つ
fn allowable_deflection(active: &[ActiveCriterion], lx: f64) -> Option<f64> {
    active
        .iter()
        .filter_map(|c| c.allowable_deflection(lx))
        .reduce(f64::min)
}

/// たわみ式の形状係数 λ⁴/(1+λ⁴)
fn shape(lambda: f64) -> f64 {
    lambda.powi(4) / (1.0 + lambda.powi(4))
}

/// たわみに効く条件の必要厚さ [mm]。K·δe = 許容たわみ を t について解く。
///   δe = (1/32)·(λ⁴/(1+λ⁴))·w·lx⁴ / (E·t³),  w = γ₁·t + wp
///   t³ = B·(γ₁·t + wp),  B = K·(λ⁴/(1+λ⁴))·lx⁴ / (32·E·許容たわみ)
fn deflection_thickness(basis: &Basis, lx: f64, lambda: f64, allowable: f64) -> f64 {
    let b =
        basis.amplification * shape(lambda) * lx.powi(4) / (32.0 * basis.young_modulus * allowable);
    solve_thickness(b * basis.rc_unit_weight, b * basis.wp)
}

/// 表18-1（周辺固定）の最小厚さ [mm]。
///   t = 0.02·((λ−0.7)/(λ−0.6))·(1 + wp/10 + lx/10000)·lx    （wp は kN/m²、lx は mm）
/// たわみに依らず、短辺スパン・辺長比・wp だけで決まる厚さの下限
fn table18_thickness(basis: &Basis, lx: f64, lambda: f64) -> f64 {
    table18_aspect_factor(lambda) * table18_span_term(basis, lx)
}

/// 表18-1 の λ の項 (λ−0.7)/(λ−0.6)。λ ≥ 1 で単調に増え、λ = 1 で 0.75、λ → ∞ で 1 に近づく
fn table18_aspect_factor(lambda: f64) -> f64 {
    (lambda - 0.7) / (lambda - 0.6)
}

/// 表18-1 の λ を除いた項 0.02·(1 + wp/10 + lx/10000)·lx [mm]。λ の項の上限が 1 なので、
/// 短辺 lx を固定して長辺を伸ばしたときの厚さの漸近値でもある
fn table18_span_term(basis: &Basis, lx: f64) -> f64 {
    let wp = basis.wp * 1e3; // N/mm² → kN/m²（表18-1 の式の単位）
    0.02 * (1.0 + wp / 10.0 + lx / 10_000.0) * lx
}

/// 表18-1 の等厚曲線の、x [mm] での y [mm]。r は x を短辺と見たときに λ の項が取るべき値:
/// - r ≥ 1: t が漸近値以上で、どんな y でもこの厚さで足りる。境界は無い
/// - r ≥ f(1): 境界は x が短辺の側（y ≥ x）。λ の項が一次分数式なので
///   f(λ) = r は λ = (0.7 − 0.6·r)/(1 − r) と閉形式で解ける
/// - r < f(1): 正方形 (x, x) でも t を超えるので、境界は y が短辺の側（y < x）。二分法で解く
fn table18_curve_y(basis: &Basis, t: f64, x: f64) -> Option<f64> {
    let r = t / table18_span_term(basis, x);
    if r >= 1.0 {
        None
    } else if r >= table18_aspect_factor(1.0) {
        Some(x * (0.7 - 0.6 * r) / (1.0 - r))
    } else {
        Some(table18_short_span(basis, t, x))
    }
}

/// 長辺 x [mm] を固定して、表18-1 の厚さがちょうど t になる短辺 y [mm]（0 < y < x）。
///
/// h(y) = f(x/y)·0.02·(1 + wp/10 + y/10000)·y は y ∈ (0, x] で狭義単調増加: λ の項の減り
/// |dln f/dln y| ≤ 5/6（y = x で最大）は、span の項の増え dln/dln y ≥ 1 に必ず負ける。
/// h(0⁺) = 0 < t < h(x) なので解は一つで、区間を半分ずつ詰めれば倍精度の限界まで届く
fn table18_short_span(basis: &Basis, t: f64, x: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, x);
    for _ in 0..60 {
        let y = 0.5 * (lo + hi);
        if table18_thickness(basis, y, x / y) < t {
            lo = y;
        } else {
            hi = y;
        }
    }
    0.5 * (lo + hi)
}

/// たわみに効く条件の等厚曲線の定数 C。判定式を厚さ t 固定で整理した右辺（ADR-0001）:
///
///   lx^n·λ⁴/(1+λ⁴) = C = 32·q·E·t³ / (K·w),  w = γ₁·t + wp
///
/// 許容たわみが lx の何乗かは条件による（変形角は q·lx で n = 3、q は比で C は mm³。
/// たわみ絶対量は q で n = 4、q は mm で C は mm⁴）。係数をここ 1 か所に持ち、条件ごとの曲線がずれないようにする
fn curve_constant(basis: &Basis, t: f64, allowable_coefficient: f64) -> f64 {
    let w = basis.rc_unit_weight * t + basis.wp;
    32.0 * allowable_coefficient * basis.young_modulus * t.powi(3) / (basis.amplification * w)
}

/// たわみ絶対量の等厚曲線の、x [mm] での y [mm]。境界が無い x では `None`。
///
/// lx⁴·λ⁴/(1+λ⁴) は lx = min(x, y)、λ = max(x, y)/min(x, y) を戻すと x⁴y⁴/(x⁴+y⁴) で、x と y の
/// 入れ替えに対して対称。y について解くと r = C/x⁴ として y = x·(r/(1−r))^(1/4) の一つの式で
/// 両枝が書ける。左辺は y について単調に増え、上限は x⁴ なので、r ≥ 1 では境界が無い
fn absolute_curve_y(c: f64, x: f64) -> Option<f64> {
    let r = c / x.powi(4);
    (r < 1.0).then(|| x * (r / (1.0 - r)).powf(0.25))
}

/// 変形角の等厚曲線の、x [mm] での y [mm]。境界が無い x では `None`。
///
/// lx³·λ⁴/(1+λ⁴) = C を場合分けして解く。左辺は y について単調に増え、上限は x³ なので:
/// - r = C/x³ ≥ 1: どんな y でも左辺が C に届かず、この厚さで足りる。境界は無い
/// - 1/2 ≤ r < 1: x が短辺（y ≥ x）。λ⁴ = r/(1−r) と閉形式で解ける
/// - r < 1/2: x が長辺（y < x）。u = x/y が u/(1+u⁴) = r を満たし、Newton 法で解く
fn ratio_curve_y(c: f64, x: f64) -> Option<f64> {
    let r = c / x.powi(3);
    if r >= 1.0 {
        None
    } else if r >= 0.5 {
        Some(x * (r / (1.0 - r)).powf(0.25))
    } else {
        Some(x / solve_aspect(r))
    }
}

/// u/(1+u⁴) = r（0 < r < 1/2）の u ≥ 1 での唯一の解。左辺は u ≥ 1 で単調に減るので解は一つ。
/// Newton 法で、初期値は遠方の近似 u ≈ r^(−1/3)
fn solve_aspect(r: f64) -> f64 {
    let mut u = r.powf(-1.0 / 3.0);
    for _ in 0..100 {
        let denom = 1.0 + u.powi(4);
        let g = u / denom - r;
        let dg = (1.0 - 3.0 * u.powi(4)) / (denom * denom);
        let next = (u - g / dg).max(1.0);
        if (next - u).abs() < 1e-12 * u.max(1.0) {
            return next;
        }
        u = next;
    }
    u
}

/// t³ − p·t − q = 0（p, q ≥ 0）の唯一の正の根。f は t ≥ √(p/3) で凸かつ単調に増えるので、
/// その領域から始めれば Newton 法が収束する
fn solve_thickness(p: f64, q: f64) -> f64 {
    let mut t = (p / 3.0).sqrt().max(q.cbrt()) + 1.0;
    for _ in 0..100 {
        let f = t * t * t - p * t - q;
        let df = 3.0 * t * t - p;
        let next = t - f / df;
        if (next - t).abs() < 1e-9 * t.max(1.0) {
            return next;
        }
        t = next;
    }
    t
}

/// 合成（max・min）の仕組みのテスト。合成は条件の種類を見ないので、値の違う同じ種類の条件を 2 つ並べて
/// 仕組みだけを確かめる（種類をまたいだ合成は `tests/slab_thickness.rs`）
#[cfg(test)]
mod tests {
    use super::*;
    use rustruct_types::concrete::{Concrete, ConcreteKind};

    /// Fc 24、wp 2800 N/m²、K 8 の土台
    fn basis() -> Basis {
        let concrete = Concrete::new(Stress::n_per_mm2(24.0), ConcreteKind::Normal).unwrap();
        Basis {
            young_modulus: concrete.young_modulus().as_n_per_mm2(),
            rc_unit_weight: concrete.rc_unit_weight().as_kn_per_m3() * 1e-6,
            wp: 2800.0e-6,
            amplification: 8.0,
        }
    }

    /// 必要スラブ厚は有効な条件ごとの必要厚さの最大（厳しい条件が決める）。
    /// 決定条件はその最大を与えた条件で、集合の並びに依らない
    #[test]
    fn thickness_is_the_max_over_active_criteria() {
        let (lx, lambda) = (4000.0, 1.25);
        let strict = ActiveCriterion::DeflectionRatio(1.0 / 400.0);
        let loose = ActiveCriterion::DeflectionRatio(1.0 / 200.0);
        let t_strict = strict.required_thickness(&basis(), lx, lambda);
        let t_loose = loose.required_thickness(&basis(), lx, lambda);
        assert!(t_strict > t_loose, "{t_strict} <= {t_loose}");

        for order in [[strict, loose], [loose, strict]] {
            let (t, governing) = governing_thickness(&order, &basis(), lx, lambda);
            assert_eq!(t, t_strict, "{order:?}");
            assert_eq!(governing, Criterion::DeflectionRatio);
        }
    }

    /// 合成した等厚曲線は各条件の曲線の点ごとの最小（いちばん低い境界が効く）
    #[test]
    fn curve_is_the_pointwise_min_over_active_criteria() {
        let (t, x) = (200.0, 12_000.0);
        let strict = ActiveCriterion::DeflectionRatio(1.0 / 400.0);
        let loose = ActiveCriterion::DeflectionRatio(1.0 / 250.0);
        let y_strict = strict.curve_y(&basis(), t, x).expect("境界を持つ前提");
        let y_loose = loose.curve_y(&basis(), t, x).expect("境界を持つ前提");
        assert!(y_strict < y_loose, "{y_strict} >= {y_loose}");

        for order in [[strict, loose], [loose, strict]] {
            assert_eq!(
                combined_curve_y(&order, &basis(), t, x),
                Some(y_strict),
                "{order:?}"
            );
        }
    }

    /// 境界を持たない条件は合成に加わらない。合成が境界を持たないのは、有効な条件がすべて境界を持たないときだけ
    #[test]
    fn combined_curve_is_unbounded_only_when_every_criterion_is() {
        let t = 200.0;
        let bounded = ActiveCriterion::DeflectionRatio(1.0 / 400.0);
        let unbounded = ActiveCriterion::DeflectionRatio(1.0 / 250.0);
        // 厚さ 200 mm では、x = 7 m で変形角 1/400 だけが境界を持つ
        let x = 7000.0;
        let y = bounded.curve_y(&basis(), t, x).expect("境界を持つ前提");
        assert_eq!(
            unbounded.curve_y(&basis(), t, x),
            None,
            "境界を持たない前提"
        );
        for order in [[bounded, unbounded], [unbounded, bounded]] {
            assert_eq!(
                combined_curve_y(&order, &basis(), t, x),
                Some(y),
                "{order:?}"
            );
        }
        // どちらも境界を持たない x では合成も境界を持たない
        let x = 4000.0;
        assert_eq!(bounded.curve_y(&basis(), t, x), None, "境界を持たない前提");
        assert_eq!(
            combined_curve_y(&[bounded, unbounded], &basis(), t, x),
            None
        );
    }

    /// 許容たわみはたわみに効く条件の上限の最小（厳しいほうが効く）。
    /// たわみに効かない条件は加わらず、それだけの集合では許容たわみが無い
    #[test]
    fn allowable_deflection_is_the_min_over_deflection_criteria() {
        let lx = 4000.0;
        let strict = ActiveCriterion::DeflectionRatio(1.0 / 400.0);
        let loose = ActiveCriterion::DeflectionRatio(1.0 / 200.0);
        let thickness_floor = ActiveCriterion::MinThickness;
        assert_eq!(allowable_deflection(&[strict], lx), Some(10.0));
        assert_eq!(allowable_deflection(&[loose], lx), Some(20.0));
        assert_eq!(allowable_deflection(&[loose, strict], lx), Some(10.0));
        assert_eq!(
            allowable_deflection(&[loose, thickness_floor], lx),
            Some(20.0)
        );
        assert_eq!(allowable_deflection(&[thickness_floor], lx), None);
    }
}
