//! 支持条件の組（81 通り）を 21 種の型に正規化し、写し方で元に戻せる（ADR-0003）。
//! 型の 4 文字は、その型に属する組の記号のうち F < R < S の辞書順で最小のもの（GLOSSARY「支持条件の型」）。

use std::collections::BTreeSet;

use rustruct_calc::kirchhoff_plate::{Mapping, SupportType};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};

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

/// GLOSSARY と ADR-0002 が挙げる 21 種（18 種 ＋ 後回しの FRFR ＋ 不安定な RRRR・RRRS）
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
        .map(|&s| SupportType::of(s).0.symbol())
        .collect();
    let expected: BTreeSet<&str> = TYPES.into_iter().collect();
    assert_eq!(found, expected);
    assert_eq!(SupportType::ALL.len(), 21);
}

#[test]
fn the_type_symbol_is_the_smallest_in_its_orbit_and_is_its_own_representative() {
    for s in every_combination() {
        let (ty, _) = SupportType::of(s);
        assert!(
            ty.symbol() <= s.symbol().as_str(),
            "{} の型 {} が組の記号より大きい",
            s.symbol(),
            ty.symbol()
        );
        // 代表の向きの組は、それ自身を型とし写し方は恒等
        let (again, mapping) = SupportType::of(ty.representative());
        assert_eq!(again, ty);
        assert_eq!(mapping, Mapping::IDENTITY);
    }
}

#[test]
fn the_mapping_carries_the_representative_back_to_the_chosen_supports() {
    for s in every_combination() {
        let (ty, mapping) = SupportType::of(s);
        assert_eq!(mapping.supports(ty.representative()), s, "{}", s.symbol());
        // 辺の写しと組の写しは同じことを言う
        for edge in Edge::ALL {
            assert_eq!(
                s.at(edge),
                ty.representative().at(mapping.edge(edge)),
                "{} の {edge}",
                s.symbol()
            );
        }
        // 角は会う 2 辺の写しで決まる
        for corner in Corner::ALL {
            let (ex, ey) = corner.edges();
            let mapped = mapping.corner(corner).edges();
            let pair = [mapping.edge(ex), mapping.edge(ey)];
            assert!(pair.contains(&mapped.0) && pair.contains(&mapped.1));
        }
    }
}

#[test]
fn symmetric_combinations_normalize_with_the_identity() {
    for support in SUPPORTS {
        let (_, mapping) = SupportType::of(Supports::all(support));
        assert_eq!(mapping, Mapping::IDENTITY);
    }
}

#[test]
fn a_rotated_combination_maps_edges_and_positions() {
    // 固定 1 辺が x = Lx にある組。代表 FRRR は固定を x = 0 に置くので、x を裏返す写し
    let s = Supports {
        x0: Support::Free,
        y0: Support::Free,
        x_lx: Support::Fixed,
        y_ly: Support::Free,
    };
    let (ty, mapping) = SupportType::of(s);
    assert_eq!(ty.symbol(), "FRRR");
    assert_eq!(mapping.edge(Edge::XLx), Edge::X0);
    assert_eq!(mapping.edge(Edge::Y0), Edge::Y0);
    assert!(mapping.flip_x && !mapping.flip_y && !mapping.transpose);

    // 固定 1 辺が y = 0 にある組。x と y を入れ替える写し
    let s = Supports {
        x0: Support::Free,
        y0: Support::Fixed,
        x_lx: Support::Free,
        y_ly: Support::Free,
    };
    let (ty, mapping) = SupportType::of(s);
    assert_eq!(ty.symbol(), "FRRR");
    assert_eq!(mapping.edge(Edge::Y0), Edge::X0);
    assert!(mapping.transpose && !mapping.flip_x && !mapping.flip_y);
}
