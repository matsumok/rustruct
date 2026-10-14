//! 鉄筋の呼び名の諸元が JIS G 3112 の表と一致する。期待値は `docs/materials/adr/0002` の表の写し。

mod common;
use common::assert_close;
use rustruct_types::rebar::{BarDesignation, RebarKind};

/// 呼び名, 公称直径 d (mm), 公称周長 l (mm), 公称断面積 S (mm²), 単位質量 w (kg/m)
const JIS_TABLE: [(BarDesignation, f64, f64, f64, f64); 16] = [
    (BarDesignation::D4, 4.23, 13.3, 14.05, 0.110),
    (BarDesignation::D5, 5.29, 16.6, 21.98, 0.173),
    (BarDesignation::D6, 6.35, 20.0, 31.67, 0.249),
    (BarDesignation::D8, 7.94, 24.9, 49.51, 0.389),
    (BarDesignation::D10, 9.53, 29.9, 71.33, 0.560),
    (BarDesignation::D13, 12.7, 39.9, 126.7, 0.995),
    (BarDesignation::D16, 15.9, 50.0, 198.6, 1.56),
    (BarDesignation::D19, 19.1, 60.0, 286.5, 2.25),
    (BarDesignation::D22, 22.2, 69.8, 387.1, 3.04),
    (BarDesignation::D25, 25.4, 79.8, 506.7, 3.98),
    (BarDesignation::D29, 28.6, 89.9, 642.4, 5.04),
    (BarDesignation::D32, 31.8, 99.9, 794.2, 6.23),
    (BarDesignation::D35, 34.9, 109.7, 956.6, 7.51),
    (BarDesignation::D38, 38.1, 119.7, 1140.0, 8.95),
    (BarDesignation::D41, 41.3, 129.8, 1340.0, 10.5),
    (BarDesignation::D51, 50.8, 159.6, 2027.0, 15.9),
];

#[test]
fn every_designation_matches_the_jis_table() {
    for (designation, d, l, s, w) in JIS_TABLE {
        let name = designation.name();
        assert_close(
            designation.nominal_diameter().as_mm(),
            d,
            &format!("{name} の公称直径"),
        );
        assert_close(
            designation.nominal_perimeter().as_mm(),
            l,
            &format!("{name} の公称周長"),
        );
        assert_close(
            designation.nominal_area().as_mm2(),
            s,
            &format!("{name} の公称断面積"),
        );
        assert_close(
            designation.unit_mass().as_kg_per_m(),
            w,
            &format!("{name} の単位質量"),
        );
    }
}

#[test]
fn the_table_covers_every_designation() {
    let listed: Vec<_> = JIS_TABLE.iter().map(|row| row.0).collect();
    assert_eq!(BarDesignation::ALL.to_vec(), listed);
}

#[test]
fn designations_are_named_as_in_jis() {
    assert_eq!(BarDesignation::D13.name(), "D13");
    assert_eq!(BarDesignation::D51.name(), "D51");
}

#[test]
fn kinds_are_the_ten_jis_symbols() {
    let symbols: Vec<_> = RebarKind::ALL.iter().map(|g| g.symbol()).collect();
    assert_eq!(
        symbols,
        [
            "SD295", "SD345", "SD390", "SD490", "SD590A", "SD590B", "SD685A", "SD685B", "SD685R",
            "SD785R"
        ]
    );
}
