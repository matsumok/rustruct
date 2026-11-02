//! Kirchhoff 板の計算核の入口（不安定な型と不正な入力を弾く、分割数、グリッドと辺の評価の並び）と、四辺固定
//! （FFFF）の板の辺の値。入口 `solve`（有限要素）からだけ確かめる。19 型すべてに共通の照合（境界条件、釣り合い、
//! 対称、分割数を倍にしたときの変化、帯板の極限、ν の換算）は `kirchhoff_plate_all_types.rs`、級数解との照合は
//! `kirchhoff_plate_series_reference.rs`

mod common;
use common::{assert_within, at, integrated_reaction, plate};
use rustruct_calc::kirchhoff_plate::{
    DIVISIONS, EdgeSample, Plate, PlateError, SupportType, mesh_divisions, solve,
};
use rustruct_types::kirchhoff_plate::{Edge, Support, Supports};
use rustruct_types::quantity::{AreaLoad, Length, Stress};

const FFFF: Supports = Supports::all(Support::Fixed);
/// 三辺固定、y = Ly の辺が自由
const FFFR: Supports = Supports {
    x0: Support::Fixed,
    y0: Support::Fixed,
    x_lx: Support::Fixed,
    y_ly: Support::Free,
};
/// x = 0 と y = 0 の辺が固定、x = Lx と y = Ly の辺が単純支持
const FFSS: Supports = Supports {
    x0: Support::Fixed,
    y0: Support::Fixed,
    x_lx: Support::SimplySupported,
    y_ly: Support::SimplySupported,
};

fn fixed(lx: f64, ly: f64, nu: f64, p: f64) -> Plate {
    plate(lx, ly, nu, p, FFFF)
}

#[test]
fn unstable_types_are_refused_and_the_other_nineteen_solve() {
    for ty in SupportType::ALL {
        let p = plate(2000.0, 3000.0, 0.2, 10.0, ty.representative());
        let result = solve(&p);
        if ty.is_stable() {
            let s = result.unwrap_or_else(|e| panic!("{}: {e}", ty.symbol()));
            assert_eq!(s.support_type(), ty);
            let centre = at(&s, 1000.0, 1500.0);
            assert!(
                centre.w.as_mm().is_finite() && centre.w.as_mm() > 0.0,
                "{}",
                ty.symbol()
            );
        } else {
            assert_eq!(result.err(), Some(PlateError::Unstable(ty)));
        }
    }
}

#[test]
fn invalid_inputs_are_refused() {
    use rustruct_calc::kirchhoff_plate::InvalidInput;
    let bad = |f: fn(&mut Plate)| {
        let mut p = fixed(1000.0, 1000.0, 0.0, 1.0);
        f(&mut p);
        solve(&p).err()
    };
    assert_eq!(
        bad(|p| p.lx = Length::mm(0.0)),
        Some(PlateError::InvalidInput(InvalidInput::Lx))
    );
    assert_eq!(
        bad(|p| p.ly = Length::mm(-1.0)),
        Some(PlateError::InvalidInput(InvalidInput::Ly))
    );
    assert_eq!(
        bad(|p| p.thickness = Length::mm(f64::NAN)),
        Some(PlateError::InvalidInput(InvalidInput::Thickness))
    );
    assert_eq!(
        bad(|p| p.young_modulus = Stress::n_per_mm2(0.0)),
        Some(PlateError::InvalidInput(InvalidInput::YoungModulus))
    );
    assert_eq!(
        bad(|p| p.load = AreaLoad::n_per_mm2(f64::INFINITY)),
        Some(PlateError::InvalidInput(InvalidInput::Load))
    );
    // 荷重 0 と負（上向き）は不正ではない
    assert!(bad(|p| p.load = AreaLoad::n_per_mm2(0.0)).is_none());
    assert!(bad(|p| p.load = AreaLoad::n_per_mm2(-1.0)).is_none());
}

#[test]
fn mesh_divisions_grow_with_the_aspect_ratio_and_stay_even() {
    assert_eq!(mesh_divisions(1.0, DIVISIONS), (DIVISIONS, DIVISIONS));
    let (nx, ny) = mesh_divisions(2.0, DIVISIONS);
    assert_eq!(nx, DIVISIONS);
    assert!(ny > DIVISIONS && ny.is_multiple_of(2));
    let (nx4, ny4) = mesh_divisions(4.0, DIVISIONS);
    assert_eq!(nx4, DIVISIONS);
    assert!(ny4 > ny);
    assert_eq!(mesh_divisions(0.25, DIVISIONS), (ny4, nx4));
    // 縦横比が大きいと長い辺の分割数は上限で止まる
    let (_, ny100) = mesh_divisions(100.0, DIVISIONS);
    let (_, ny1000) = mesh_divisions(1000.0, DIVISIONS);
    assert_eq!(ny100, ny1000);
    assert!(ny100.is_multiple_of(2));
}

#[test]
fn the_edges_of_a_fixed_plate_support_it_at_their_middle() {
    // 反力は支える向き（正）。辺の中央で見る。釣り合いは `kirchhoff_plate_all_types.rs`
    for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (4000.0, 2000.0)] {
        let s = solve(&fixed(lx, ly, 0.15, 8.0)).unwrap();
        for e in Edge::ALL {
            let mid = s.edge(e, 2)[1];
            assert!(
                mid.reaction.as_n_per_mm() > 0.0,
                "{e} の反力が支える向きでない"
            );
        }
    }
}

#[test]
fn the_edge_samples_integrate_to_the_total_reaction() {
    // 辺の反力の分布は節点の間で線形なので、細かい台形則の積分は合計に 1e-4 で一致する（節点の位置と評価点が
    // 揃わない分だけ差が出る）
    let s = solve(&fixed(2000.0, 3000.0, 0.2, 10.0)).unwrap();
    for e in Edge::ALL {
        let total = s.total_reaction(e).as_n();
        assert_within(
            integrated_reaction(&s, e, 2000),
            total,
            1e-4 * total.abs(),
            &format!("{e} の合計"),
        );
    }
}

#[test]
fn the_edge_maximum_of_a_square_plate_is_at_the_middle_of_the_edge() {
    // 四辺固定と四辺単純支持。単純支持の辺の曲げモーメントは 0 なので、位置は固定の板だけ見る
    let l = 2000.0;
    for supports in [FFFF, Supports::all(Support::SimplySupported)] {
        let s = solve(&plate(l, l, 0.0, 10.0, supports)).unwrap();
        let what = supports.symbol();
        for e in Edge::ALL {
            let m = s.edge_maximum(e, 100);
            let mid = s.edge(e, 100)[50];
            assert_eq!(
                m.reaction.along.as_mm(),
                l / 2.0,
                "{what} {e} の反力の最大の位置"
            );
            assert_eq!(m.reaction.value, mid.reaction, "{what} {e} の反力の最大");
            assert_eq!(
                m.shear.along.as_mm(),
                l / 2.0,
                "{what} {e} のせん断力の最大の位置"
            );
            assert_eq!(m.shear.value, mid.shear, "{what} {e} のせん断力の最大");
            if supports == FFFF {
                assert_eq!(
                    m.moment.along.as_mm(),
                    l / 2.0,
                    "{what} {e} の曲げモーメントの最大の位置"
                );
                assert_eq!(
                    m.moment.value, mid.moment,
                    "{what} {e} の曲げモーメントの最大"
                );
            }
        }
        // 辺の上の曲げモーメントとせん断力は、内部の評価と同じ導関数から求めるので同じ値。単純支持の辺の
        // 曲げモーメントは境界条件どおり 0 で返す（`kirchhoff_plate_simply_supported.rs`）
        let edge = s.edge(Edge::X0, 2)[1];
        let point = at(&s, 0.0, l / 2.0);
        if supports == FFFF {
            assert_eq!(edge.moment, point.mx, "{what} 辺の中央の Mx");
        }
        assert_eq!(edge.shear, point.qx, "{what} 辺の中央の Qx");
    }
}

/// 評価点のうち、`f` の絶対値が最大のもの
fn largest(samples: &[EdgeSample], f: fn(&EdgeSample) -> f64) -> EdgeSample {
    *samples
        .iter()
        .max_by(|a, b| f(a).abs().total_cmp(&f(b).abs()))
        .unwrap()
}

#[test]
fn with_a_free_edge_the_edge_maximum_of_shear_and_reaction_skips_five_percent_at_each_end() {
    // ADR-0008: 自由辺のある組では、せん断力と辺の反力の辺の最大値を両端から辺長の 5 % を除いた範囲で取り、
    // 曲げモーメントは隅まで含める。辺を 100 等分した評価点なら、両端の 5 点ずつを除いた 6 番目から 96 番目
    let (lx, ly) = (2000.0, 3000.0);
    let s = solve(&plate(lx, ly, 0.2, 10.0, FFFR)).unwrap();
    for e in Edge::ALL {
        let samples = s.edge(e, 100);
        let inner = &samples[5..=95];
        let m = s.edge_maximum(e, 100);
        let shear = largest(inner, |s| s.shear.as_n_per_mm());
        let reaction = largest(inner, |s| s.reaction.as_n_per_mm());
        let moment = largest(&samples, |s| s.moment.as_nmm_per_mm());
        assert_eq!(m.shear.value, shear.shear, "{e} のせん断力の最大");
        assert_eq!(m.reaction.value, reaction.reaction, "{e} の反力の最大");
        assert_eq!(m.moment.value, moment.moment, "{e} の曲げモーメントの最大");
        if m.shear.value.as_n_per_mm() != 0.0 {
            assert_eq!(m.shear.along, shear.along, "{e} のせん断力の最大の位置");
        }
        if m.reaction.value.as_n_per_mm() != 0.0 {
            assert_eq!(m.reaction.along, reaction.along, "{e} の反力の最大の位置");
        }
    }
    // x = 0 の固定辺は y = Ly の自由辺と隅で会う。全長ならせん断力と反力の最大は隅の発散で、曲げモーメントの
    // 最大は自由辺の端から 5 % 以内にある
    let x0 = s.edge(Edge::X0, 100);
    let m = s.edge_maximum(Edge::X0, 100);
    assert!(
        largest(&x0, |s| s.shear.as_n_per_mm())
            .shear
            .as_n_per_mm()
            .abs()
            > m.shear.value.as_n_per_mm().abs()
    );
    assert!(
        largest(&x0, |s| s.reaction.as_n_per_mm())
            .reaction
            .as_n_per_mm()
            .abs()
            > m.reaction.value.as_n_per_mm().abs()
    );
    assert!(m.moment.along.as_mm() > 0.95 * ly, "{:?}", m.moment);
}

#[test]
fn without_a_free_edge_the_edge_maximum_runs_over_the_whole_edge() {
    // 自由辺の無い組（FFSS）は隅まで含める（ADR-0008）。x = Lx の単純支持の辺のせん断力の最大は辺の端にある
    let (lx, ly) = (2000.0, 3000.0);
    let s = solve(&plate(lx, ly, 0.2, 10.0, FFSS)).unwrap();
    for e in Edge::ALL {
        let samples = s.edge(e, 100);
        let m = s.edge_maximum(e, 100);
        let shear = largest(&samples, |s| s.shear.as_n_per_mm());
        let reaction = largest(&samples, |s| s.reaction.as_n_per_mm());
        assert_eq!(
            (m.shear.value, m.shear.along),
            (shear.shear, shear.along),
            "{e} のせん断力"
        );
        assert_eq!(
            (m.reaction.value, m.reaction.along),
            (reaction.reaction, reaction.along),
            "{e} の反力"
        );
    }
    let along = s.edge_maximum(Edge::XLx, 100).shear.along.as_mm();
    assert!(along < 0.05 * ly || along > 0.95 * ly, "{along}");
}

#[test]
fn a_point_is_near_a_corner_when_both_x_and_y_are_within_five_percent_of_an_end() {
    // ADR-0008: 隅から辺長の 5 % 以内の点は、x と y のどちらも辺長の 5 % より端に寄った点。ちょうど 5 % の点は
    // 含まない
    let (lx, ly) = (2000.0, 3000.0);
    let s = solve(&fixed(lx, ly, 0.0, 10.0)).unwrap();
    let near = |x: f64, y: f64| s.near_corner(Length::mm(x), Length::mm(y));
    for (x, y) in [
        (0.0, 0.0),
        (99.0, 149.0),
        (1901.0, 149.0),
        (1901.0, 2851.0),
        (99.0, 3000.0),
    ] {
        assert!(near(x, y), "({x}, {y})");
    }
    for (x, y) in [
        (99.0, 1500.0),
        (1000.0, 149.0),
        (100.0, 150.0),
        (1900.0, 2850.0),
        // 21 × 21 のグリッドの隅から 1 つ目の点
        (lx / 20.0, ly / 20.0),
        (lx * 19.0 / 20.0, ly * 19.0 / 20.0),
    ] {
        assert!(!near(x, y), "({x}, {y})");
    }
}

#[test]
fn the_grid_runs_over_the_whole_plate_with_x_changing_fastest() {
    let (lx, ly) = (2000.0, 3000.0);
    let s = solve(&fixed(lx, ly, 0.0, 10.0)).unwrap();
    let g = s.grid(4, 6);
    assert_eq!(g.len(), 5 * 7);
    assert_eq!((g[0].x.as_mm(), g[0].y.as_mm()), (0.0, 0.0));
    assert_eq!((g[1].x.as_mm(), g[1].y.as_mm()), (500.0, 0.0));
    assert_eq!((g[5].x.as_mm(), g[5].y.as_mm()), (0.0, 500.0));
    assert_eq!((g[34].x.as_mm(), g[34].y.as_mm()), (lx, ly));
    assert_eq!(g[2 + 3 * 5], at(&s, 1000.0, 1500.0));
    // 板の外は評価しない
    assert!(s.at(Length::mm(-1.0), Length::mm(0.0)).is_none());
    assert!(s.at(Length::mm(0.0), Length::mm(ly + 1.0)).is_none());
}
