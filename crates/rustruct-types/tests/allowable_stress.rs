//! 許容応力度。期待値は RC 規準 表 6.1〜6.3（`docs/materials/adr/0003`）の式の手計算と、表 6.2 の写し。

mod common;
use common::assert_close;
use rustruct_types::allowable_stress::AllowableStress;
use rustruct_types::concrete::{BarPosition, Concrete, ConcreteKind};
use rustruct_types::quantity::Stress;
use rustruct_types::rebar::{BarDesignation, RebarKind};

fn normal(fc: f64) -> Concrete {
    Concrete::new(Stress::n_per_mm2(fc), ConcreteKind::Normal).unwrap()
}

fn assert_pair(actual: AllowableStress, long: f64, short: f64, what: &str) {
    assert_close(
        actual.long_term.as_n_per_mm2(),
        long,
        &format!("{what} の長期"),
    );
    assert_close(
        actual.short_term.as_n_per_mm2(),
        short,
        &format!("{what} の短期"),
    );
}

#[test]
fn concrete_compression_is_a_third_of_fc_and_twice_that_short_term() {
    // 長期 Fc/3、短期は長期の 2 倍
    assert_pair(
        normal(24.0).allowable_compression(),
        8.0,
        16.0,
        "Fc 24 の圧縮",
    );
    assert_pair(
        normal(36.0).allowable_compression(),
        12.0,
        24.0,
        "Fc 36 の圧縮",
    );
}

#[test]
fn concrete_shear_is_the_smaller_of_fc_over_30_and_0_49_plus_fc_over_100() {
    // Fc 21 で 2 つの式が等しくなる。それより下は Fc/30、上は 0.49 + Fc/100
    assert_pair(normal(18.0).allowable_shear(), 0.6, 0.9, "Fc 18 のせん断"); // 0.6 < 0.67
    assert_pair(
        normal(24.0).allowable_shear(),
        0.73,
        1.095,
        "Fc 24 のせん断",
    ); // 0.8 > 0.73
    assert_pair(
        normal(60.0).allowable_shear(),
        1.09,
        1.635,
        "Fc 60 のせん断",
    ); // 2.0 > 1.09
}

#[test]
fn bond_of_top_bars_is_the_smaller_of_fc_over_15_and_0_9_plus_2fc_over_75() {
    // Fc 22.5 で等しくなる
    assert_pair(
        normal(18.0).allowable_bond(BarPosition::Top),
        1.2,
        1.8,
        "Fc 18 の上端筋",
    ); // 1.2 < 1.38
    assert_pair(
        normal(24.0).allowable_bond(BarPosition::Top),
        1.54,
        2.31,
        "Fc 24 の上端筋",
    ); // 1.6 > 1.54
}

#[test]
fn bond_of_other_bars_is_the_smaller_of_fc_over_10_and_1_35_plus_fc_over_25() {
    // Fc 22.5 で等しくなる
    assert_pair(
        normal(18.0).allowable_bond(BarPosition::Other),
        1.8,
        2.7,
        "Fc 18 のその他の鉄筋",
    ); // 1.8 < 2.07
    assert_pair(
        normal(24.0).allowable_bond(BarPosition::Other),
        2.31,
        3.465,
        "Fc 24 のその他の鉄筋",
    ); // 2.4 > 2.31
}

/// 種類, 長期 引張・圧縮（D25 以下, D29 以上）, 長期 せん断補強, 短期 引張・圧縮, 短期 せん断補強。表 6.2 の写し
const REBAR_TABLE: [(RebarKind, f64, f64, f64, f64, f64); 4] = [
    (RebarKind::Sd295, 195.0, 195.0, 195.0, 295.0, 295.0),
    (RebarKind::Sd345, 215.0, 195.0, 195.0, 345.0, 345.0),
    (RebarKind::Sd390, 215.0, 195.0, 195.0, 390.0, 390.0),
    (RebarKind::Sd490, 215.0, 195.0, 195.0, 490.0, 490.0),
];

#[test]
fn rebar_allowable_stresses_follow_the_rc_standard_table() {
    for (kind, long, long_d29, long_shear, short, short_shear) in REBAR_TABLE {
        let k = kind.symbol();
        assert_pair(
            kind.allowable_tension_compression(BarDesignation::D25),
            long,
            short,
            &format!("{k} D25 の引張・圧縮"),
        );
        assert_pair(
            kind.allowable_tension_compression(BarDesignation::D29),
            long_d29,
            short,
            &format!("{k} D29 の引張・圧縮"),
        );
        assert_pair(
            kind.allowable_shear_reinforcement(),
            long_shear,
            short_shear,
            &format!("{k} のせん断補強"),
        );
    }
}

#[test]
fn the_table_covers_every_kind() {
    let listed: Vec<_> = REBAR_TABLE.iter().map(|row| row.0).collect();
    assert_eq!(RebarKind::ALL.to_vec(), listed);
}

#[test]
fn every_designation_from_d29_up_takes_the_reduced_long_term_value() {
    for d in BarDesignation::ALL.iter().copied() {
        let expected = if d.nominal_diameter() >= BarDesignation::D29.nominal_diameter() {
            195.0
        } else {
            215.0
        };
        assert_close(
            RebarKind::Sd345
                .allowable_tension_compression(d)
                .long_term
                .as_n_per_mm2(),
            expected,
            &format!("SD345 {} の長期", d.name()),
        );
    }
}
