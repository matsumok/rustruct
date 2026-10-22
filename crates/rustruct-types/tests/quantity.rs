//! Kirchhoff 板の曲げで足した物理量（ADR-0004）。生成の単位と取り出しの単位の換算と、ポアソン比の範囲。
//! 期待値は換算の係数（10³ 倍など）の手計算。

mod common;
use common::assert_close;
use rustruct_types::quantity::{
    AreaLoad, Force, ForcePerLength, MomentPerLength, PoissonRatio, PoissonRatioError,
};

#[test]
fn area_load_converts_between_kn_per_m2_and_n_per_mm2() {
    // 1 kN/m² = 10³ N / 10⁶ mm² = 10⁻³ N/mm²
    assert_close(AreaLoad::kn_per_m2(10.0).as_n_per_mm2(), 0.01, "10 kN/m²");
    assert_close(AreaLoad::n_per_mm2(0.01).as_kn_per_m2(), 10.0, "0.01 N/mm²");
}

#[test]
fn moment_per_length_converts_between_knm_per_m_and_nmm_per_mm() {
    // 1 kN·m/m = 10³ N × 10³ mm / 10³ mm = 10³ N·mm/mm
    assert_close(
        MomentPerLength::knm_per_m(2.5).as_nmm_per_mm(),
        2500.0,
        "2.5 kN·m/m",
    );
    assert_close(
        MomentPerLength::nmm_per_mm(2500.0).as_knm_per_m(),
        2.5,
        "2500 N·mm/mm",
    );
}

#[test]
fn force_per_length_converts_between_kn_per_m_and_n_per_mm() {
    // 1 kN/m = 10³ N / 10³ mm = 1 N/mm
    assert_close(ForcePerLength::kn_per_m(3.0).as_n_per_mm(), 3.0, "3 kN/m");
    assert_close(ForcePerLength::n_per_mm(3.0).as_kn_per_m(), 3.0, "3 N/mm");
}

#[test]
fn force_converts_between_kn_and_n() {
    assert_close(Force::kn(1.5).as_n(), 1500.0, "1.5 kN");
    assert_close(Force::n(1500.0).as_kn(), 1.5, "1500 N");
}

#[test]
fn poisson_ratio_accepts_zero_up_to_below_half() {
    assert_eq!(PoissonRatio::new(0.0).unwrap().value(), 0.0);
    assert_eq!(PoissonRatio::new(0.3).unwrap().value(), 0.3);
    assert_eq!(
        PoissonRatio::new(0.5_f64.next_down()).unwrap().value(),
        0.5_f64.next_down()
    );
}

#[test]
fn poisson_ratio_rejects_values_outside_the_range() {
    for bad in [-0.1, 0.5, 0.6, f64::NAN, f64::INFINITY] {
        assert_eq!(
            PoissonRatio::new(bad),
            Err(PoissonRatioError::OutOfRange),
            "ν = {bad}"
        );
    }
}

#[test]
fn poisson_ratio_zero_is_the_default() {
    assert_eq!(PoissonRatio::default().value(), 0.0);
}
