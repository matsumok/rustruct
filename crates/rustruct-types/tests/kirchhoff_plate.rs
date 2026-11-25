//! Kirchhoff 板の支持条件の組。辺は座標で呼び、4 文字の記号は x = 0 の辺から反時計回り
//! （`docs/tools/kirchhoff-plate/GLOSSARY.md` の「支持条件の組」）。板の材料から導く E と ν
//! （同じ GLOSSARY の「板の材料」）。

use rustruct_types::concrete::{Concrete, ConcreteKind};
use rustruct_types::kirchhoff_plate::{Corner, Edge, PlateMaterial, Support, Supports};
use rustruct_types::quantity::{PoissonRatio, Stress};

#[test]
fn supports_are_read_back_by_edge() {
    let s = Supports {
        x0: Support::Fixed,
        y0: Support::SimplySupported,
        x_lx: Support::Free,
        y_ly: Support::Fixed,
    };
    assert_eq!(s.at(Edge::X0), Support::Fixed);
    assert_eq!(s.at(Edge::Y0), Support::SimplySupported);
    assert_eq!(s.at(Edge::XLx), Support::Free);
    assert_eq!(s.at(Edge::YLy), Support::Fixed);
}

#[test]
fn symbol_runs_counterclockwise_from_the_x0_edge() {
    let s = Supports {
        x0: Support::Fixed,
        y0: Support::SimplySupported,
        x_lx: Support::Free,
        y_ly: Support::Fixed,
    };
    assert_eq!(s.symbol(), "FSRF");
    assert_eq!(Supports::all(Support::SimplySupported).symbol(), "SSSS");
}

#[test]
fn symbols_round_trip() {
    for s in [Support::Fixed, Support::SimplySupported, Support::Free] {
        assert_eq!(Support::from_symbol(s.symbol()), Some(s));
    }
    assert_eq!(Support::from_symbol('X'), None);
}

#[test]
fn every_corner_names_its_two_edges() {
    assert_eq!(Corner::X0Y0.edges(), (Edge::X0, Edge::Y0));
    assert_eq!(Corner::XLxY0.edges(), (Edge::XLx, Edge::Y0));
    assert_eq!(Corner::XLxYLy.edges(), (Edge::XLx, Edge::YLy));
    assert_eq!(Corner::X0YLy.edges(), (Edge::X0, Edge::YLy));
    assert_eq!(Edge::ALL.len(), 4);
    assert_eq!(Corner::ALL.len(), 4);
}

#[test]
fn a_combination_has_a_free_edge_when_any_one_edge_is_free() {
    assert!(!Supports::all(Support::Fixed).has_free_edge());
    assert!(!Supports::all(Support::SimplySupported).has_free_edge());
    assert!(Supports::all(Support::Free).has_free_edge());
    let one_free = Supports {
        x0: Support::Fixed,
        y0: Support::SimplySupported,
        x_lx: Support::Fixed,
        y_ly: Support::Free,
    };
    assert!(one_free.has_free_edge());
}

#[test]
fn concrete_gives_its_young_modulus_and_the_poisson_ratio_held_with_it() {
    let concrete = Concrete::new(Stress::n_per_mm2(24.0), ConcreteKind::Normal).unwrap();
    let material = PlateMaterial::Concrete {
        concrete,
        poisson_ratio: PoissonRatio::new(0.2).unwrap(),
    };
    assert_eq!(material.young_modulus(), concrete.young_modulus());
    assert_eq!(material.poisson_ratio().value(), 0.2);
}

#[test]
fn steel_gives_the_steel_values() {
    assert_eq!(
        PlateMaterial::Steel.young_modulus().as_n_per_mm2(),
        205000.0
    );
    assert_eq!(PlateMaterial::Steel.poisson_ratio().value(), 0.3);
}

#[test]
fn direct_input_gives_the_values_given() {
    let material = PlateMaterial::Direct {
        young_modulus: Stress::n_per_mm2(22700.0),
        poisson_ratio: PoissonRatio::new(0.15).unwrap(),
    };
    assert_eq!(material.young_modulus().as_n_per_mm2(), 22700.0);
    assert_eq!(material.poisson_ratio().value(), 0.15);
}
