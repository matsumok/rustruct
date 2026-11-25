//! 鋼材の鋼種・基準強度 F・E・ν。期待値は ADR materials-0004 の写し。

use rustruct_types::quantity::Length;
use rustruct_types::steel::{self, SteelGrade};

/// 鋼種, 記号, t ≤ 40 の F, 40 < t ≤ 100 の F（N/mm²）
const TABLE: [(SteelGrade, &str, f64, Option<f64>); 12] = [
    (SteelGrade::Ss400, "SS400", 235.0, Some(215.0)),
    (SteelGrade::Sn400A, "SN400A", 235.0, Some(215.0)),
    (SteelGrade::Sn400B, "SN400B", 235.0, Some(215.0)),
    (SteelGrade::Sn400C, "SN400C", 235.0, Some(215.0)),
    (SteelGrade::Sn490B, "SN490B", 325.0, Some(295.0)),
    (SteelGrade::Sn490C, "SN490C", 325.0, Some(295.0)),
    (SteelGrade::Sm490A, "SM490A", 325.0, Some(295.0)),
    (SteelGrade::Stkr400, "STKR400", 235.0, None),
    (SteelGrade::Stkr490, "STKR490", 325.0, None),
    (SteelGrade::Bcr295, "BCR295", 295.0, None),
    (SteelGrade::Bcp235, "BCP235", 235.0, Some(215.0)),
    (SteelGrade::Bcp325, "BCP325", 325.0, Some(295.0)),
];

fn f_at(grade: SteelGrade, t_mm: f64) -> Option<f64> {
    grade
        .standard_strength(Length::mm(t_mm))
        .map(|f| f.as_n_per_mm2())
}

#[test]
fn the_list_holds_the_twelve_grades_in_table_order() {
    let listed: Vec<_> = TABLE.iter().map(|row| row.0).collect();
    assert_eq!(SteelGrade::ALL.to_vec(), listed);
}

#[test]
fn grades_and_symbols_look_each_other_up() {
    for (grade, symbol, _, _) in TABLE {
        assert_eq!(grade.symbol(), symbol);
        assert_eq!(SteelGrade::from_symbol(symbol), Some(grade));
    }
    assert_eq!(SteelGrade::from_symbol("SN490A"), None);
    assert_eq!(SteelGrade::from_symbol("ss400"), None);
}

#[test]
fn standard_strength_follows_the_thickness_ranges() {
    for (grade, symbol, up_to_40, over_40) in TABLE {
        assert_eq!(f_at(grade, 40.0), Some(up_to_40), "{symbol} の t = 40");
        assert_eq!(f_at(grade, 41.0), over_40, "{symbol} の t = 41");
        assert_eq!(f_at(grade, 100.0), over_40, "{symbol} の t = 100");
    }
}

#[test]
fn standard_strength_is_none_outside_the_ranges() {
    for grade in SteelGrade::ALL.iter().copied() {
        let symbol = grade.symbol();
        for t in [0.0, -1.0, 100.1, f64::NAN, f64::INFINITY] {
            assert_eq!(f_at(grade, t), None, "{symbol} の t = {t}");
        }
    }
    for grade in [SteelGrade::Stkr400, SteelGrade::Stkr490, SteelGrade::Bcr295] {
        assert_eq!(f_at(grade, 41.0), None, "{} の t = 41", grade.symbol());
    }
}

#[test]
fn young_modulus_and_poisson_ratio_are_shared_by_every_grade() {
    assert_eq!(steel::young_modulus().as_n_per_mm2(), 205000.0);
    assert_eq!(steel::poisson_ratio().value(), 0.3);
}
