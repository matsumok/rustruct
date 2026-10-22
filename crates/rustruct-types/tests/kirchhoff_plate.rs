//! Kirchhoff 板の支持条件の組。辺は座標で呼び、4 文字の記号は x = 0 の辺から反時計回り
//! （`docs/tools/kirchhoff-plate/GLOSSARY.md` の「支持条件の組」）。

use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};

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
