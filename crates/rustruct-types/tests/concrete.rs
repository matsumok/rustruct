//! コンクリートの単位体積重量とヤング係数。期待値は `docs/materials/adr/0001` の表と、式の手計算。

mod common;
use common::assert_close;
use rustruct_types::concrete::{Concrete, ConcreteError, ConcreteKind, ConcreteWarning};
use rustruct_types::quantity::Stress;

fn normal(fc: f64) -> Result<Concrete, ConcreteError> {
    Concrete::new(Stress::n_per_mm2(fc), ConcreteKind::Normal)
}

/// 境界のすぐ上。f64 で表せる、`x` より大きい最小の数（36 なら 36.00000000000001）。
/// 「≦」と「<」の取り違えを、境界を挟む2点で確かめるのに使う
fn just_above(x: f64) -> f64 {
    x.next_up()
}

/// 境界のすぐ下。f64 で表せる、`x` より小さい最大の数
fn just_below(x: f64) -> f64 {
    x.next_down()
}

/// Fc (N/mm²), γ (kN/m³), γ₁ (kN/m³)。各区分の境界と、そのすぐ上
fn table() -> [(f64, f64, f64); 10] {
    [
        (18.0, 23.0, 24.0),
        (36.0, 23.0, 24.0),
        (just_above(36.0), 23.5, 24.5),
        (48.0, 23.5, 24.5),
        (just_above(48.0), 24.0, 25.0),
        (60.0, 24.0, 25.0),
        (just_above(60.0), 24.5, 25.5),
        (80.0, 24.5, 25.5),
        (just_above(80.0), 25.0, 26.0),
        (100.0, 25.0, 26.0),
    ]
}

#[test]
fn unit_weights_follow_the_fc_table() {
    for (fc, gamma, gamma1) in table() {
        let c = normal(fc).unwrap();
        assert_close(
            c.unit_weight().as_kn_per_m3(),
            gamma,
            &format!("Fc {fc} の γ"),
        );
        assert_close(
            c.rc_unit_weight().as_kn_per_m3(),
            gamma1,
            &format!("Fc {fc} の γ₁"),
        );
    }
}

#[test]
fn young_modulus_follows_the_rc_standard_formula() {
    // E = 3.35×10⁴ × (γ/24)² × (Fc/60)^(1/3)
    // Fc 60, γ 24 は係数がすべて 1 で 33500
    assert_close(
        normal(60.0).unwrap().young_modulus().as_n_per_mm2(),
        33500.0,
        "Fc 60",
    );
    // Fc 24, γ 23: 33500 × (23/24)² × 0.4^(1/3)
    assert_close(
        normal(24.0).unwrap().young_modulus().as_n_per_mm2(),
        22668.94590387348,
        "Fc 24",
    );
    // Fc 100, γ 25: 33500 × (25/24)² × (100/60)^(1/3)
    assert_close(
        normal(100.0).unwrap().young_modulus().as_n_per_mm2(),
        43097.4847006717,
        "Fc 100",
    );
}

#[test]
fn fc_must_be_positive_and_finite() {
    for fc in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            normal(fc).unwrap_err(),
            ConcreteError::NotPositive,
            "Fc {fc}"
        );
    }
}

#[test]
fn fc_within_18_to_100_has_no_warning() {
    for fc in [18.0, 24.0, 100.0] {
        assert_eq!(normal(fc).unwrap().warning(), None, "Fc {fc}");
    }
}

#[test]
fn fc_below_18_is_computed_with_a_warning() {
    for fc in [just_below(18.0), 12.0] {
        let c = normal(fc).unwrap();
        assert_eq!(c.warning(), Some(ConcreteWarning::BelowMinimum), "Fc {fc}");
        assert_close(
            c.unit_weight().as_kn_per_m3(),
            23.0,
            &format!("Fc {fc} の γ"),
        );
    }
}

#[test]
fn fc_above_100_is_computed_with_the_last_range_and_a_warning() {
    for fc in [just_above(100.0), 120.0] {
        let c = normal(fc).unwrap();
        assert_eq!(c.warning(), Some(ConcreteWarning::AboveTable), "Fc {fc}");
        assert_close(
            c.unit_weight().as_kn_per_m3(),
            25.0,
            &format!("Fc {fc} の γ"),
        );
        assert_close(
            c.rc_unit_weight().as_kn_per_m3(),
            26.0,
            &format!("Fc {fc} の γ₁"),
        );
    }
}

#[test]
fn concrete_keeps_its_fc_and_kind() {
    let c = normal(24.0).unwrap();
    assert_eq!(c.fc(), Stress::n_per_mm2(24.0));
    assert_eq!(c.kind(), ConcreteKind::Normal);
}
