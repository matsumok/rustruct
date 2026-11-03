//! 支持条件の組（81 通り）を 21 種の型に正規化する（ADR kirchhoff-plate-0003）。写し方の中は計算核の中の
//! テスト（`support_type.rs`）が見る。
//! 型の 4 文字は、その型に属する組の記号のうち F < R < S の辞書順で最小のもの（GLOSSARY「支持条件の型」）。

use std::collections::BTreeSet;

use rustruct_calc::kirchhoff_plate::SupportType;
use rustruct_types::kirchhoff_plate::{Support, Supports};

const SUPPORTS: [Support; 3] = [Support::Fixed, Support::SimplySupported, Support::Free];

fn every_combination() -> Vec<Supports> {
    let mut out = Vec::new();
    for x0 in SUPPORTS {
        for y0 in SUPPORTS {
            for x_lx in SUPPORTS {
                for y_ly in SUPPORTS {
                    out.push(Supports { x0, y0, x_lx, y_ly });
                }
            }
        }
    }
    out
}

/// GLOSSARY と ADR kirchhoff-plate-0002 が挙げる 21 種（18 種 ＋ 後回しの FRFR ＋ 不安定な RRRR・RRRS）
const TYPES: [&str; 21] = [
    "FFFF", "FFFR", "FFFS", "FFRR", "FFRS", "FFSS", "FRFR", "FRFS", "FRRR", "FRRS", "FRSR", "FRSS",
    "FSFS", "FSRS", "FSSS", "RRRR", "RRRS", "RRSS", "RSRS", "RSSS", "SSSS",
];

#[test]
fn the_81_combinations_fall_into_the_21_types() {
    let combos = every_combination();
    assert_eq!(combos.len(), 81);
    let found: BTreeSet<&str> = combos
        .iter()
        .map(|&s| SupportType::of(s).symbol())
        .collect();
    let expected: BTreeSet<&str> = TYPES.into_iter().collect();
    assert_eq!(found, expected);
    assert_eq!(SupportType::ALL.len(), 21);
}

#[test]
fn the_type_symbol_is_the_smallest_in_its_orbit_and_is_its_own_representative() {
    for s in every_combination() {
        let ty = SupportType::of(s);
        assert!(
            ty.symbol() <= s.symbol().as_str(),
            "{} の型 {} が組の記号より大きい",
            s.symbol(),
            ty.symbol()
        );
        // 代表の向きの組は、それ自身を型とする
        assert_eq!(SupportType::of(ty.representative()), ty);
    }
}

#[test]
fn only_four_free_edges_and_one_simply_supported_with_three_free_are_unstable() {
    // 四辺自由と、単純支持 1 辺と自由 3 辺は板を支えきれない（ADR kirchhoff-plate-0003）
    let unstable: Vec<&str> = SupportType::ALL
        .into_iter()
        .filter(|t| !t.is_stable())
        .map(SupportType::symbol)
        .collect();
    assert_eq!(unstable, ["RRRR", "RRRS"]);
}
