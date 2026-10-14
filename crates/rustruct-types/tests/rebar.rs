//! 鉄筋の呼び名の諸元が JIS G 3112 の表と一致する。期待値は `docs/materials/adr/0002` の表の写し。

use rustruct_types::rebar::{BarSize, RebarGrade};

/// 呼び名, 公称直径 d (mm), 公称周長 l (mm), 公称断面積 S (mm²), 単位質量 w (kg/m)
const JIS_TABLE: [(BarSize, f64, f64, f64, f64); 16] = [
    (BarSize::D4, 4.23, 13.3, 14.05, 0.110),
    (BarSize::D5, 5.29, 16.6, 21.98, 0.173),
    (BarSize::D6, 6.35, 20.0, 31.67, 0.249),
    (BarSize::D8, 7.94, 24.9, 49.51, 0.389),
    (BarSize::D10, 9.53, 29.9, 71.33, 0.560),
    (BarSize::D13, 12.7, 39.9, 126.7, 0.995),
    (BarSize::D16, 15.9, 50.0, 198.6, 1.56),
    (BarSize::D19, 19.1, 60.0, 286.5, 2.25),
    (BarSize::D22, 22.2, 69.8, 387.1, 3.04),
    (BarSize::D25, 25.4, 79.8, 506.7, 3.98),
    (BarSize::D29, 28.6, 89.9, 642.4, 5.04),
    (BarSize::D32, 31.8, 99.9, 794.2, 6.23),
    (BarSize::D35, 34.9, 109.7, 956.6, 7.51),
    (BarSize::D38, 38.1, 119.7, 1140.0, 8.95),
    (BarSize::D41, 41.3, 129.8, 1340.0, 10.5),
    (BarSize::D51, 50.8, 159.6, 2027.0, 15.9),
];

/// 単位の換算で生じる2進の誤差だけを許す。丸めの幅（表の最下位桁）よりはるかに小さい
fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 1e-12,
        "{what}: {actual} != {expected}"
    );
}

#[test]
fn every_bar_size_matches_the_jis_table() {
    for (size, d, l, s, w) in JIS_TABLE {
        let name = size.name();
        assert_close(
            size.nominal_diameter().as_mm(),
            d,
            &format!("{name} の公称直径"),
        );
        assert_close(
            size.nominal_perimeter().as_mm(),
            l,
            &format!("{name} の公称周長"),
        );
        assert_close(
            size.nominal_area().as_mm2(),
            s,
            &format!("{name} の公称断面積"),
        );
        assert_close(
            size.unit_mass().as_kg_per_m(),
            w,
            &format!("{name} の単位質量"),
        );
    }
}

#[test]
fn the_table_covers_every_bar_size() {
    let listed: Vec<_> = JIS_TABLE.iter().map(|row| row.0).collect();
    assert_eq!(BarSize::ALL.to_vec(), listed);
}

#[test]
fn bar_sizes_are_named_as_in_jis() {
    assert_eq!(BarSize::D13.name(), "D13");
    assert_eq!(BarSize::D51.name(), "D51");
}

#[test]
fn grades_are_the_ten_jis_symbols() {
    let symbols: Vec<_> = RebarGrade::ALL.iter().map(|g| g.symbol()).collect();
    assert_eq!(
        symbols,
        [
            "SD295", "SD345", "SD390", "SD490", "SD590A", "SD590B", "SD685A", "SD685B", "SD685R",
            "SD785R"
        ]
    );
}
