//! 必要スラブ厚さの計算核。入口の `shared_values`・`evaluate`・`iso_curves` からだけ確かめる。
//! 言葉は `docs/tools/slab-thickness/GLOSSARY.md`。手計算の値は電卓で独立に確かめたもの。
//! 条件の既定は Fc 24、仕上げ荷重 1000 N/m²、積載荷重 1800 N/m²、変形増大係数 8、変形角 1/400。

use rustruct_calc::slab_thickness::{
    Conditions, CriteriaSet, Criterion, Evaluation, InvalidInput, Spans, evaluate,
};
use rustruct_types::concrete::{Concrete, ConcreteKind};
use rustruct_types::quantity::{AreaLoad, Length, Stress};

/// 変形角 1/denom だけの集合
fn deflection_ratio(denom: f64) -> CriteriaSet {
    CriteriaSet {
        deflection_ratio: Some(1.0 / denom),
        ..CriteriaSet::default()
    }
}

/// たわみ絶対量 limit [mm] だけの集合
fn deflection_absolute(limit: f64) -> CriteriaSet {
    CriteriaSet {
        deflection_absolute: Some(Length::mm(limit)),
        ..CriteriaSet::default()
    }
}

/// 変形角とたわみ絶対量の両方の集合
fn both_criteria(denom: f64, limit: f64) -> CriteriaSet {
    CriteriaSet {
        deflection_ratio: Some(1.0 / denom),
        deflection_absolute: Some(Length::mm(limit)),
        ..CriteriaSet::default()
    }
}

/// 表18-1 の最小厚さだけの集合（変形角は空欄）
fn min_thickness_only() -> CriteriaSet {
    CriteriaSet {
        min_thickness: true,
        ..CriteriaSet::default()
    }
}

/// 3 つそろった集合
fn all_three() -> CriteriaSet {
    CriteriaSet {
        min_thickness: true,
        ..both_criteria(400.0, 20.0)
    }
}

fn concrete(fc: f64) -> Concrete {
    Concrete::new(Stress::n_per_mm2(fc), ConcreteKind::Normal).unwrap()
}

fn base() -> Conditions {
    Conditions {
        concrete: concrete(24.0),
        finish_load: AreaLoad::n_per_m2(1000.0),
        live_load: AreaLoad::n_per_m2(1800.0),
        amplification: 8.0,
        criteria: deflection_ratio(400.0),
    }
}

fn with(criteria: CriteriaSet) -> Conditions {
    Conditions { criteria, ..base() }
}

/// スパンは m
fn spans(x: f64, y: f64) -> Spans {
    Spans {
        x: Length::mm(x * 1000.0),
        y: Length::mm(y * 1000.0),
    }
}

/// スパンは m
fn at(conditions: &Conditions, x: f64, y: f64) -> Evaluation {
    evaluate(conditions, spans(x, y)).unwrap()
}

/// 必要スラブ厚 [mm]。スパンは m
fn t_req(conditions: &Conditions, x: f64, y: f64) -> f64 {
    at(conditions, x, y).required_thickness.as_mm()
}

/// 手計算の既知の場合（電卓で独立に確かめた値）:
/// E = 3.35e4·(23/24)²·(24/60)^(1/3) ≈ 22668.9 N/mm²
/// λ = 5.0/4.0 = 1.25
/// t³ = B·(γ₁t + wp), B = K·λ⁴/(1+λ⁴)·lx³/(32·E·c) ≈ 2.0029e8
///   → t_req ≈ 101.62 mm
/// 増大後たわみ = 許容たわみ = lx·c = 4000/400 = 10.0 mm
#[test]
fn known_case_matches_hand_calculation() {
    let shared = rustruct_calc::slab_thickness::shared_values(&base()).unwrap();
    assert!(
        (shared.young_modulus.as_n_per_mm2() - 22668.9).abs() < 1.0,
        "E = {:?}",
        shared.young_modulus
    );

    let ev = at(&base(), 4.0, 5.0);
    assert!(
        (ev.aspect_ratio - 1.25).abs() < 1e-12,
        "λ = {}",
        ev.aspect_ratio
    );
    assert!(
        (ev.short_span.as_mm() - 4000.0).abs() < 1e-9,
        "lx = {:?}",
        ev.short_span
    );
    let t = ev.required_thickness.as_mm();
    assert!((t - 101.62).abs() < 0.05, "t_req = {t}");
    let allowable = ev
        .allowable_deflection
        .expect("変形角が許容たわみを与える")
        .as_mm();
    assert!((allowable - 10.0).abs() < 1e-9, "許容たわみ = {allowable}");
    // 変形角が決定条件なら、t_req はその条件をちょうど満たす厚さ:
    // 増大後たわみ = 許容たわみ（表18-1 が決めると等号にならない）
    assert_eq!(ev.governing, Criterion::DeflectionRatio);
    let amplified = ev.amplified_deflection.as_mm();
    assert!(
        (amplified - allowable).abs() < 1e-6,
        "増大後たわみ = {amplified}"
    );
    // 増大後たわみ = 変形増大係数 × δe
    assert!((ev.elastic_deflection.as_mm() * 8.0 - amplified).abs() < 1e-9);
    // 全荷重 = γ₁·t_req + 仕上げ + 積載（Fc 24 では γ₁ = 24 kN/m³）
    let expected_w = 24_000.0 * (t / 1000.0) + 2800.0;
    assert!(
        (ev.total_load.as_n_per_m2() - expected_w).abs() < 1e-6,
        "w = {:?}",
        ev.total_load
    );
}

/// 表18-1（周辺固定）の最小厚さの、手計算の既知の場合（電卓で独立に確かめた値）。
/// wp は N/m² から kN/m² へ、lx は m から mm へ直して刻む。
///   t = 0.02·((λ−0.7)/(λ−0.6))·(1 + wp/10 + lx/10000)·lx
/// - (4.0, 5.0)・wp = 2800 N/m²: 0.02·(0.55/0.65)·(1+0.28+0.4)·4000 = 113.723 mm
/// - (6.0, 6.0)・wp = 2800 N/m²: λ = 1 の下端 0.02·0.75·(1+0.28+0.6)·6000 = 169.2 mm
/// - (4.0, 8.0)・wp = 5000 N/m²: 0.02·(1.3/1.4)·(1+0.5+0.4)·4000 = 141.143 mm
#[test]
fn min_thickness_matches_hand_calculation() {
    let cases = [
        (4.0, 5.0, 1000.0, 1800.0, 113.723_076_923),
        (6.0, 6.0, 1000.0, 1800.0, 169.2),
        (4.0, 8.0, 2000.0, 3000.0, 141.142_857_142),
    ];
    for (x, y, finish, live, expected) in cases {
        let ev = at(
            &Conditions {
                finish_load: AreaLoad::n_per_m2(finish),
                live_load: AreaLoad::n_per_m2(live),
                ..with(min_thickness_only())
            },
            x,
            y,
        );
        let t = ev.required_thickness.as_mm();
        assert!(
            (t - expected).abs() < 1e-6,
            "({x}, {y}) wp = {} の t_req = {t}（期待 {expected}）",
            finish + live
        );
        assert_eq!(ev.governing, Criterion::MinThickness);
    }
}

/// 表18-1 は厚さの下限だけを課し、たわみには効かない（変形増大係数にも Fc にも依らない）
#[test]
fn min_thickness_ignores_deflection_conditions() {
    let base = with(min_thickness_only());
    let t = t_req(&base, 4.0, 5.0);
    for conditions in [
        Conditions {
            amplification: 32.0,
            ..base
        },
        Conditions {
            concrete: concrete(42.0),
            ..base
        },
    ] {
        let other = t_req(&conditions, 4.0, 5.0);
        assert!((other - t).abs() < 1e-9, "{other} != {t}");
    }
}

/// 表18-1 を足した集合の必要スラブ厚は、変形角だけの厚さと表18-1 の厚さの最大。
/// 決定条件は厳しいほうで、どちらが決めるかは条件（変形増大係数）とスパンで入れ替わる
#[test]
fn adding_min_thickness_takes_the_max_of_both_criteria() {
    let mut governings = Vec::new();
    for amplification in [8.0, 16.0] {
        for (x, y) in [(4.0, 5.0), (3.0, 3.0), (8.0, 9.0), (12.0, 4.0), (2.0, 18.0)] {
            let at = |criteria| {
                at(
                    &Conditions {
                        amplification,
                        criteria,
                        ..base()
                    },
                    x,
                    y,
                )
            };
            let ratio_only = at(deflection_ratio(400.0)).required_thickness.as_mm();
            let table_only = at(min_thickness_only()).required_thickness.as_mm();
            let both = at(CriteriaSet {
                min_thickness: true,
                ..deflection_ratio(400.0)
            });
            assert!(
                (both.required_thickness.as_mm() - ratio_only.max(table_only)).abs() < 1e-9,
                "K = {amplification} ({x}, {y}): 合成 {:?} が max({ratio_only}, {table_only}) でない",
                both.required_thickness
            );
            assert_eq!(
                both.governing,
                if table_only > ratio_only {
                    Criterion::MinThickness
                } else {
                    Criterion::DeflectionRatio
                },
                "K = {amplification} ({x}, {y})"
            );
            governings.push(both.governing);
        }
    }
    // どちらが決める場合も実際に通っている（最大の検査が片側だけで済んでいない）
    assert!(
        governings.contains(&Criterion::MinThickness),
        "{governings:?}"
    );
    assert!(
        governings.contains(&Criterion::DeflectionRatio),
        "{governings:?}"
    );
}

/// 変形角と表18-1 が入れ替わる向き。表18-1 の厚さは lx² の項を持ち、たわみの厚さより速く増えるので、
/// 短いスパンでは変形角が、長いスパンでは表18-1 が決める（逆にはならない）。変形増大係数 12 では
/// lx 8〜12 m の間で入れ替わる
#[test]
fn min_thickness_takes_over_from_the_deflection_ratio_as_the_span_grows() {
    let conditions = Conditions {
        amplification: 12.0,
        criteria: CriteriaSet {
            min_thickness: true,
            ..deflection_ratio(400.0)
        },
        ..base()
    };
    let governing: Vec<_> = (1..=40)
        .map(|i| at(&conditions, i as f64 * 0.5, i as f64 * 0.5 * 1.25).governing)
        .collect();
    let switch = governing
        .iter()
        .position(|&g| g == Criterion::MinThickness)
        .expect("長いスパンでは表18-1 が決める");
    assert!(switch > 0, "最も短いスパンから表18-1 が決めている");
    assert!(
        governing[..switch]
            .iter()
            .all(|&g| g == Criterion::DeflectionRatio)
            && governing[switch..]
                .iter()
                .all(|&g| g == Criterion::MinThickness),
        "変形角 → 表18-1 の一度だけの入れ替わりでない: {governing:?}"
    );
}

/// 表18-1 が決める点では、必要スラブ厚は変形角が求めるより厚いので、増大後たわみは許容たわみに届かない
/// （合成では「ちょうど満たす」が等号にならない）。たわみに効く条件が 1 つも無ければ許容たわみそのものが無い
#[test]
fn deflection_is_slack_where_min_thickness_governs() {
    let ev = at(
        &with(CriteriaSet {
            min_thickness: true,
            ..deflection_ratio(400.0)
        }),
        4.0,
        5.0,
    );
    assert_eq!(ev.governing, Criterion::MinThickness);
    let allowable = ev
        .allowable_deflection
        .expect("変形角があるので許容たわみがある");
    assert!(
        ev.amplified_deflection < allowable,
        "増大後たわみ {:?} が許容たわみ {allowable:?} 未満でない",
        ev.amplified_deflection
    );

    // 表18-1 だけの集合では、たわみの上限を課す条件が無い
    let ev = at(&with(min_thickness_only()), 4.0, 5.0);
    assert_eq!(ev.allowable_deflection, None);
    assert!(ev.amplified_deflection.as_mm() > 0.0);
}

/// (Lx, Ly) と (Ly, Lx) で結果が変わらない（x と y のどちら向きに入れても同じ）
#[test]
fn swapping_spans_gives_identical_result() {
    assert_eq!(at(&base(), 4.0, 5.0), at(&base(), 5.0, 4.0));
}

/// スパンが伸びれば必要スラブ厚は単調に増える
#[test]
fn required_thickness_increases_with_span() {
    let mut prev = 0.0;
    for span in [3.0_f64, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0] {
        let t = t_req(&base(), span, span * 1.25);
        assert!(t > prev, "スパン {span} m: t_req {t} <= {prev}");
        prev = t;
    }
}

/// 片方のスパンを固定してもう片方を伸ばすと、必要スラブ厚は単調に増える。等厚曲線の図で、表示範囲の
/// 隅だけから範囲の中の厚さを決めるときの前提。条件の組ごとに両方向を広く確かめる
/// （合成は最大なので、条件を足しても単調性は保たれる）。変形角とたわみ絶対量が入れ替わる 8 m の前後をまたぐ
#[test]
fn required_thickness_is_monotonic_in_each_span() {
    let ratio_and_table = CriteriaSet {
        min_thickness: true,
        ..deflection_ratio(400.0)
    };
    for criteria in [
        deflection_ratio(400.0),
        min_thickness_only(),
        ratio_and_table,
        both_criteria(400.0, 20.0),
        all_three(),
    ] {
        let conditions = with(criteria);
        for fixed in [2.0_f64, 4.0, 7.0, 9.0, 12.0, 14.0] {
            let (mut prev_x, mut prev_y) = (0.0, 0.0);
            for i in 0..=100 {
                let span = 1.0 + i as f64 * 0.2; // 1〜21 m
                let tx = t_req(&conditions, span, fixed);
                let ty = t_req(&conditions, fixed, span);
                assert!(
                    tx > prev_x,
                    "{criteria:?}・Ly = {fixed} 固定・Lx = {span}: {tx} <= {prev_x}"
                );
                assert!(
                    ty > prev_y,
                    "{criteria:?}・Lx = {fixed} 固定・Ly = {span}: {ty} <= {prev_y}"
                );
                (prev_x, prev_y) = (tx, ty);
            }
        }
    }
}

/// Fc が上がれば（E が上がり）必要スラブ厚は単調に減る。Fc 36 を超えると γ と γ₁ が 0.5 kN/m³ 上がり、
/// E は (γ/24)² で上がる一方、自重も増える。E の上がりのほうが効き、36 と 42 の間でも減る
#[test]
fn required_thickness_decreases_with_fc() {
    let mut prev = f64::INFINITY;
    for fc in [18.0_f64, 21.0, 24.0, 27.0, 30.0, 36.0, 42.0, 48.0, 60.0] {
        let t = t_req(
            &Conditions {
                concrete: concrete(fc),
                ..base()
            },
            4.0,
            5.0,
        );
        assert!(t < prev, "Fc {fc}: t_req {t} >= {prev}");
        prev = t;
    }
}

/// 自重は Fc の区分の γ₁ で算定する。Fc 42 は γ = 23.5、γ₁ = 24.5 kN/m³ の区分
#[test]
fn self_weight_uses_the_rc_unit_weight_of_the_fc_class() {
    let ev = at(
        &Conditions {
            concrete: concrete(42.0),
            ..base()
        },
        4.0,
        5.0,
    );
    let expected_w = 24_500.0 * (ev.required_thickness.as_mm() / 1000.0) + 2800.0;
    assert!(
        (ev.total_load.as_n_per_m2() - expected_w).abs() < 1e-6,
        "w = {:?}",
        ev.total_load
    );
}

/// 不正な入力は理由ごとに弾く。条件欄の誤りはスパンの誤りより先に言う
#[test]
fn invalid_inputs_are_rejected() {
    let cases = [
        (
            Conditions {
                finish_load: AreaLoad::n_per_m2(-1.0),
                ..base()
            },
            InvalidInput::FinishLoad,
        ),
        (
            Conditions {
                live_load: AreaLoad::n_per_m2(-100.0),
                ..base()
            },
            InvalidInput::LiveLoad,
        ),
        (
            Conditions {
                live_load: AreaLoad::n_per_m2(f64::NAN),
                ..base()
            },
            InvalidInput::LiveLoad,
        ),
        (
            Conditions {
                amplification: 0.0,
                ..base()
            },
            InvalidInput::Amplification,
        ),
        (
            with(CriteriaSet {
                deflection_ratio: Some(0.0),
                ..CriteriaSet::default()
            }),
            InvalidInput::DeflectionRatio,
        ),
        (
            with(CriteriaSet {
                deflection_ratio: Some(-1.0 / 400.0),
                ..CriteriaSet::default()
            }),
            InvalidInput::DeflectionRatio,
        ),
    ];
    for (conditions, reason) in cases {
        assert_eq!(evaluate(&conditions, spans(0.0, 5.0)), Err(reason));
        assert_eq!(evaluate(&conditions, spans(4.0, 5.0)), Err(reason));
    }
    assert_eq!(evaluate(&base(), spans(0.0, 5.0)), Err(InvalidInput::SpanX));
    assert_eq!(
        evaluate(&base(), spans(4.0, f64::NAN)),
        Err(InvalidInput::SpanY)
    );
}

/// 荷重は 0 でよい（条件欄を空にしていく途中の状態）
#[test]
fn zero_loads_are_accepted() {
    let conditions = Conditions {
        finish_load: AreaLoad::n_per_m2(0.0),
        live_load: AreaLoad::n_per_m2(0.0),
        ..base()
    };
    let t = t_req(&conditions, 4.0, 5.0);
    assert!(t.is_finite() && t > 0.0, "t_req = {t}");
}

/// クライテリアが 1 つも有効でなければ算定できない（判定の無い図を描かせない）
#[test]
fn empty_criteria_set_is_rejected() {
    assert_eq!(
        evaluate(&with(CriteriaSet::default()), spans(4.0, 5.0)),
        Err(InvalidInput::NoCriteria)
    );
}

/// 決定条件はどの条件が必要スラブ厚を決めたかを言う。変形角だけの集合では、どのスパンでも変形角
#[test]
fn governing_criterion_is_reported() {
    for (x, y) in [(4.0, 5.0), (3.0, 3.0), (12.0, 4.0), (2.0, 18.0)] {
        assert_eq!(
            at(&base(), x, y).governing,
            Criterion::DeflectionRatio,
            "({x}, {y})"
        );
    }
}

/// たわみ絶対量だけの、手計算の既知の場合（電卓で独立に確かめた値）:
/// 許容たわみ = 20.0 mm（スパンに依らない）
/// E ≈ 22668.95 N/mm²、λ = 1.25、λ⁴/(1+λ⁴) ≈ 0.7094211
/// B = K·(λ⁴/(1+λ⁴))·lx⁴/(32·E·20) ≈ 1.00144e8
/// t³ = B·(γ₁t + wp) → t³ − 2403.45·t − 280402 = 0 → t_req ≈ 77.58 mm
#[test]
fn absolute_deflection_matches_hand_calculation() {
    let ev = at(&with(deflection_absolute(20.0)), 4.0, 5.0);
    let t = ev.required_thickness.as_mm();
    assert!((t - 77.58).abs() < 0.05, "t_req = {t}");
    // 許容たわみはスパンに依らず、入れた絶対量そのもの（たわみに効く条件なので値がある）
    let allowable = ev
        .allowable_deflection
        .expect("絶対量が許容たわみを与える")
        .as_mm();
    assert!((allowable - 20.0).abs() < 1e-9, "許容たわみ = {allowable}");
    // t_req は条件をちょうど満たす厚さ: 増大後たわみ = 許容たわみ
    let amplified = ev.amplified_deflection.as_mm();
    assert!(
        (amplified - 20.0).abs() < 1e-6,
        "増大後たわみ = {amplified}"
    );
    assert_eq!(ev.governing, Criterion::DeflectionAbsolute);
}

/// 変形角と絶対量は、同じ許容たわみを表す組では同じ答えになる（lx = 4 m では 1/200 と 20 mm が同じ 20 mm）。
/// 2 つの条件が別の式で書かれていても、同じ判定式に帰着することを確かめる
#[test]
fn absolute_and_ratio_agree_where_they_coincide() {
    let by_absolute = t_req(&with(deflection_absolute(20.0)), 4.0, 5.0);
    let by_ratio = t_req(&with(deflection_ratio(200.0)), 4.0, 5.0);
    assert!(
        (by_absolute - by_ratio).abs() < 1e-9,
        "絶対量 {by_absolute} / 変形角 {by_ratio}"
    );
}

/// 絶対量が正の有限値でなければ算定できない
#[test]
fn non_positive_absolute_deflection_is_rejected() {
    for bad in [0.0, -20.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            evaluate(&with(deflection_absolute(bad)), spans(4.0, 5.0)),
            Err(InvalidInput::DeflectionAbsolute),
            "絶対量 {bad}"
        );
    }
}

/// 両方を入れると、決定条件はスパンで入れ替わる。許容たわみは変形角が lx/400、絶対量が 20 mm 一定なので、
/// lx < 8 m では変形角が、lx > 8 m では絶対量が厳しい（決める）
#[test]
fn governing_criterion_switches_with_span() {
    let both = with(both_criteria(400.0, 20.0));
    for (x, y, expected) in [
        (4.0, 5.0, Criterion::DeflectionRatio),
        (10.0, 12.0, Criterion::DeflectionAbsolute),
    ] {
        let ev = at(&both, x, y);
        assert_eq!(ev.governing, expected, "({x}, {y})");

        // 合成の必要スラブ厚は条件ごとの必要厚さの最大（厳しいほうが残る）
        let t_ratio = t_req(&with(deflection_ratio(400.0)), x, y);
        let t_absolute = t_req(&with(deflection_absolute(20.0)), x, y);
        assert_eq!(ev.required_thickness.as_mm(), t_ratio.max(t_absolute));
        // 許容たわみは最小（厳しいほうが効く）
        let expected = (ev.short_span.as_mm() / 400.0).min(20.0);
        let allowable = ev
            .allowable_deflection
            .expect("たわみに効く条件がある")
            .as_mm();
        assert!(
            (allowable - expected).abs() < 1e-9,
            "許容たわみ = {allowable}（期待 {expected}）"
        );
    }
}

/// 3 つそろった集合でも、決定条件は必要厚さの最大を与えた条件。スパンを掃くと決める条件が実際に移る
#[test]
fn governing_criterion_tracks_the_max_across_three_criteria() {
    let singles = [
        (deflection_ratio(400.0), Criterion::DeflectionRatio),
        (deflection_absolute(20.0), Criterion::DeflectionAbsolute),
        (min_thickness_only(), Criterion::MinThickness),
    ];
    let mut seen = Vec::new();
    for i in 0..=60 {
        let span = 1.0 + i as f64 * 0.4; // 1〜25 m
        let ev = at(&with(all_three()), span, span * 1.25);
        let combined = ev.required_thickness.as_mm();

        // 条件ごとに単独で解いた必要厚さの最大が合成の答えで、決定条件はその最大を与えた条件
        let mut best = (f64::NEG_INFINITY, Criterion::DeflectionRatio);
        for (criteria, kind) in singles {
            let t = t_req(&with(criteria), span, span * 1.25);
            assert!(
                t <= combined + 1e-9,
                "スパン {span}: {kind:?} の {t} が合成の {combined} を超える"
            );
            if t > best.0 {
                best = (t, kind);
            }
        }
        assert!(
            (combined - best.0).abs() < 1e-9,
            "スパン {span}: 合成 {combined} が最大 {} と違う",
            best.0
        );
        assert_eq!(ev.governing, best.1, "スパン {span}");
        if !seen.contains(&ev.governing) {
            seen.push(ev.governing);
        }
    }
    assert!(
        seen.len() >= 2,
        "スパンを掃いても決定条件が入れ替わらない: {seen:?}"
    );
}

mod shared_values {
    use rustruct_calc::slab_thickness::{Conditions, InvalidInput, shared_values};
    use rustruct_types::quantity::AreaLoad;

    use super::{at, base, concrete};

    /// 共有値は検討点の評価が使う値と同じ（表示だけ別の経路で求めて食い違う、を防ぐ）
    #[test]
    fn matches_the_values_used_by_point_evaluation() {
        let v = shared_values(&base()).unwrap();
        let ev = at(&base(), 4.0, 5.0);
        // 全荷重 = γ₁·t_req + wp
        let w = v.rc_unit_weight.as_kn_per_m3() * 1e3 * (ev.required_thickness.as_mm() / 1000.0)
            + v.wp.as_n_per_m2();
        assert!((w - ev.total_load.as_n_per_m2()).abs() < 1e-6, "w = {w}");
    }

    /// wp は仕上げ荷重と積載荷重の和。γ₁ と E はコンクリートの値
    #[test]
    fn wp_is_the_sum_of_finish_and_live_load() {
        let v = shared_values(&base()).unwrap();
        assert!(
            (v.wp.as_n_per_m2() - 2800.0).abs() < 1e-9,
            "wp = {:?}",
            v.wp
        );
        assert_eq!(v.rc_unit_weight, concrete(24.0).rc_unit_weight());
        assert_eq!(v.young_modulus, concrete(24.0).young_modulus());
        // 荷重 0 でも求まる（条件欄を空にしていく途中の状態）
        let zero = Conditions {
            finish_load: AreaLoad::n_per_m2(0.0),
            live_load: AreaLoad::n_per_m2(0.0),
            ..base()
        };
        assert_eq!(shared_values(&zero).unwrap().wp.as_n_per_m2(), 0.0);
    }

    /// γ₁ と E は Fc の区分で変わる。Fc 42 は γ₁ = 24.5 kN/m³
    #[test]
    fn rc_unit_weight_follows_the_fc_class() {
        let v = shared_values(&Conditions {
            concrete: concrete(42.0),
            ..base()
        })
        .unwrap();
        assert!(
            (v.rc_unit_weight.as_kn_per_m3() - 24.5).abs() < 1e-12,
            "γ₁ = {:?}",
            v.rc_unit_weight
        );
        assert_eq!(v.young_modulus, concrete(42.0).young_modulus());
    }

    /// E は Fc で単調に増え、荷重に依らない
    #[test]
    fn young_modulus_grows_with_fc_and_ignores_loads() {
        let mut prev = 0.0;
        for fc in [18.0_f64, 21.0, 24.0, 30.0, 36.0, 42.0] {
            let e = shared_values(&Conditions {
                concrete: concrete(fc),
                ..base()
            })
            .unwrap()
            .young_modulus
            .as_n_per_mm2();
            assert!(e > prev, "Fc {fc}: E {e} <= {prev}");
            prev = e;
        }
        let heavy = Conditions {
            finish_load: AreaLoad::n_per_m2(5000.0),
            live_load: AreaLoad::n_per_m2(9000.0),
            ..base()
        };
        assert_eq!(
            shared_values(&base()).unwrap().young_modulus,
            shared_values(&heavy).unwrap().young_modulus
        );
    }

    /// 不正な荷重では求めない（NaN の値を表示に流さない）。変形増大係数とクライテリアは共有値に効かないので見ない
    #[test]
    fn invalid_loads_are_rejected() {
        let finish = Conditions {
            finish_load: AreaLoad::n_per_m2(-1.0),
            ..base()
        };
        assert_eq!(shared_values(&finish), Err(InvalidInput::FinishLoad));
        let live = Conditions {
            live_load: AreaLoad::n_per_m2(f64::NAN),
            ..base()
        };
        assert_eq!(shared_values(&live), Err(InvalidInput::LiveLoad));
        let no_criteria = Conditions {
            amplification: 0.0,
            criteria: Default::default(),
            ..base()
        };
        assert!(shared_values(&no_criteria).is_ok());
    }

    /// 共有値は検討点のスパンに依らない（すべての検討点に共通する）
    #[test]
    fn does_not_depend_on_spans() {
        let v = shared_values(&base()).unwrap();
        for (x, y) in [(2.0, 2.0), (4.0, 5.0), (12.0, 3.0)] {
            let ev = at(&base(), x, y);
            let w = v.rc_unit_weight.as_kn_per_m3() * ev.required_thickness.as_mm()
                + v.wp.as_n_per_m2();
            assert!((w - ev.total_load.as_n_per_m2()).abs() < 1e-6, "({x}, {y})");
        }
    }
}

mod iso_curves {
    use rustruct_calc::slab_thickness::{Conditions, CriteriaSet, InvalidInput, iso_curves};
    use rustruct_types::quantity::Length;

    use super::{
        all_three, both_criteria, deflection_absolute, deflection_ratio, min_thickness_only, t_req,
        with,
    };

    /// 曲線の性質（往復の整合・対称・厚さの順）を確かめる集合。単独の条件 3 種と、点ごとの最小で合成する集合
    fn criteria_sets() -> [CriteriaSet; 6] {
        [
            deflection_ratio(400.0),
            deflection_absolute(20.0),
            min_thickness_only(),
            both_criteria(400.0, 20.0),
            CriteriaSet {
                min_thickness: true,
                ..deflection_ratio(400.0)
            },
            all_three(),
        ]
    }

    /// x の並び [m]。1.0〜21.0 m
    fn grid() -> Vec<f64> {
        (0..=400).map(|i| 1.0 + i as f64 * 0.05).collect()
    }

    /// m → Length
    fn m(values: &[f64]) -> Vec<Length> {
        values.iter().map(|&v| Length::mm(v * 1000.0)).collect()
    }

    /// 等厚曲線群。x は m、厚さは mm。y は m で返す
    fn curves(conditions: &Conditions, xs: &[f64], ts: &[f64]) -> Vec<(f64, Vec<Option<f64>>)> {
        let ts: Vec<_> = ts.iter().map(|&t| Length::mm(t)).collect();
        iso_curves(conditions, &m(xs), &ts)
            .unwrap()
            .into_iter()
            .map(|c| {
                (
                    c.thickness.as_mm(),
                    c.ys.into_iter()
                        .map(|y| y.map(|y| y.as_mm() / 1000.0))
                        .collect(),
                )
            })
            .collect()
    }

    /// 等厚曲線群もクライテリアが 1 つも有効でなければ算定できない
    #[test]
    fn empty_criteria_set_is_rejected() {
        assert_eq!(
            iso_curves(
                &with(CriteriaSet::default()),
                &m(&grid()),
                &[Length::mm(200.0)]
            ),
            Err(InvalidInput::NoCriteria)
        );
    }

    /// 図で使う広い厚さの並び。薄い側の下限から 400 mm 超まで
    const WIDE_THICKNESSES: [f64; 7] = [80.0, 100.0, 150.0, 200.0, 300.0, 450.0, 600.0];

    /// 往復の整合: 等厚曲線の上の点を検討点として評価すると、t_req が曲線の t に一致する。
    /// 上枝（y ≥ x）と下枝（y < x）の両方を通る。合成した集合では、点ごとの最小で選ばれた条件が
    /// ちょうど t を、ほかの条件はそれ以下を求める
    #[test]
    fn points_on_curve_evaluate_back_to_curve_thickness() {
        let xs = grid();
        for criteria in criteria_sets() {
            let conditions = with(criteria);
            let curves = curves(&conditions, &xs, &WIDE_THICKNESSES);
            assert_eq!(curves.len(), WIDE_THICKNESSES.len());
            for (thickness, ys) in &curves {
                let mut checked = 0;
                for (x, y) in xs.iter().zip(ys) {
                    let Some(y) = y else { continue };
                    let t = t_req(&conditions, *x, *y);
                    assert!(
                        (t - thickness).abs() < 1e-3,
                        "{criteria:?}・t = {thickness} の曲線の上 ({x}, {y}) の評価が {t}"
                    );
                    checked += 1;
                }
                assert!(
                    checked > 50,
                    "{criteria:?}・t = {thickness} の境界のある点が少なすぎる: {checked}"
                );
            }
        }
    }

    /// 対称: 曲線が y = x に対して対称（(x, y) が乗るなら (y, x) も乗る）
    #[test]
    fn curves_are_symmetric_about_diagonal() {
        let xs = grid();
        for criteria in criteria_sets() {
            let conditions = with(criteria);
            let (_, ys) = &curves(&conditions, &xs, &[200.0])[0];
            for (x, y) in xs.iter().zip(ys) {
                let Some(y) = y else { continue };
                if *y > 21.0 {
                    continue; // 並びの外
                }
                // y を x として引き直す
                let Some(back) = curves(&conditions, &[*y], &[200.0])[0].1[0] else {
                    panic!("{criteria:?}: 鏡像の x = {y} で曲線が無い");
                };
                assert!(
                    (back - x).abs() < 1e-6,
                    "{criteria:?}: ({x}, {y}) の鏡像が ({y}, {back})"
                );
            }
        }
    }

    /// 厚さの順: 同じ x では厚い曲線ほど y が大きい（塗り分けの前提）
    #[test]
    fn thicker_curves_lie_above_thinner_ones() {
        let xs = grid();
        for criteria in criteria_sets() {
            let curves = curves(&with(criteria), &xs, &[150.0, 155.0, 200.0, 300.0]);
            for pair in curves.windows(2) {
                let ((t_lo, lo), (t_hi, hi)) = (&pair[0], &pair[1]);
                for (j, x) in xs.iter().enumerate() {
                    if let (Some(lo), Some(hi)) = (lo[j], hi[j]) {
                        assert!(
                            hi > lo,
                            "{criteria:?}: x = {x} で t = {t_hi} の y {hi} <= t = {t_lo} の y {lo}"
                        );
                    }
                }
            }
        }
    }

    /// 表18-1 を足すと、合成した等厚曲線は点ごとの最小の分だけ下がる（同じ厚さで収まるスパンが狭くなる）。
    /// 外せば元の曲線に戻る
    #[test]
    fn adding_min_thickness_lowers_the_combined_curve() {
        let xs = grid();
        let t = [200.0];
        let ratio_only = curves(&with(deflection_ratio(400.0)), &xs, &t).remove(0).1;
        let table_only = curves(&with(min_thickness_only()), &xs, &t).remove(0).1;
        let combined = curves(
            &with(CriteriaSet {
                min_thickness: true,
                ..deflection_ratio(400.0)
            }),
            &xs,
            &t,
        )
        .remove(0)
        .1;

        let mut lowered = 0;
        for (i, x) in xs.iter().enumerate() {
            let expected = match (ratio_only[i], table_only[i]) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (only, None) | (None, only) => only,
            };
            assert_eq!(combined[i], expected, "x = {x}");
            if let (Some(c), Some(r)) = (combined[i], ratio_only[i])
                && c < r
            {
                lowered += 1;
            }
        }
        assert!(lowered > 10, "曲線が下がった点が {lowered} しかない");
        // 外せば元の曲線（オフに戻すと元どおり）
        assert_eq!(
            curves(&with(deflection_ratio(400.0)), &xs, &t).remove(0).1,
            ratio_only
        );
    }

    /// 表18-1 の厚さは、短辺を固定して長辺を伸ばしても漸近値 0.02·(1 + wp/10 + lx/10000)·lx で頭打ちになる。
    /// t がその値以上の x では、どんな y でも足りる（境界が無い）
    #[test]
    fn min_thickness_curve_stops_at_its_asymptote() {
        let xs = grid();
        let t = 200.0;
        let (_, ys) = &curves(&with(min_thickness_only()), &xs, &[t])[0];
        // wp = 2800 N/m² = 2.8 kN/m²、x は m → mm
        let asymptote = |x: f64| 0.02 * (1.0 + 2.8 / 10.0 + x * 1000.0 / 10_000.0) * x * 1000.0;
        let (mut bounded, mut unbounded) = (0, 0);
        for (x, y) in xs.iter().zip(ys) {
            // 漸近値のごく近くは曲線が跳ね上がるので、少し離して見る
            if asymptote(*x) < t * 0.999 {
                assert_eq!(y, &None, "x = {x}: 漸近値 {} < t = {t}", asymptote(*x));
                unbounded += 1;
            } else if asymptote(*x) > t * 1.001 {
                // 境界は x が短辺の側（y > x）にも長辺の側（y < x）にも来るので、ここでは境界があることだけを見る
                // （位置は往復の整合で見る）
                let y = y.expect("漸近値を超える x には境界がある");
                assert!(y > 0.0 && y.is_finite(), "x = {x}: 境界 {y}");
                bounded += 1;
            }
        }
        assert!(unbounded > 10 && bounded > 10, "{unbounded} / {bounded}");
    }
}
