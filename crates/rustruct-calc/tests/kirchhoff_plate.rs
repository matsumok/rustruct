//! Kirchhoff 板の計算核の入口（不安定な型と不正な入力を弾く、グリッドと辺の評価の並び）と、四辺固定
//! （FFFF）の板の辺の値。入口 `solve`（有限要素）からだけ確かめる。19 型すべてに共通の照合（境界条件、釣り合い、
//! 対称、分割数を倍にしたときの変化、帯板の極限、ν の換算）は `kirchhoff_plate_all_types.rs`、級数解との照合は
//! `kirchhoff_plate_series_reference.rs`

mod common;
use common::{assert_within, at, integrated_reaction, plate};
use rustruct_calc::kirchhoff_plate::{EdgeSample, Plate, PlateError, SupportType, solve};
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
            let m = s.edge_maximum::<100>(e);
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
    // ADR kirchhoff-plate-0008: 自由辺のある組では、せん断力と辺の反力の辺の最大値を両端から辺長の 5 % を除いた範囲で取り、
    // 曲げモーメントは隅まで含める。辺を 100 等分した評価点なら、両端の 5 点ずつを除いた 6 番目から 96 番目
    let (lx, ly) = (2000.0, 3000.0);
    let s = solve(&plate(lx, ly, 0.2, 10.0, FFFR)).unwrap();
    for e in Edge::ALL {
        let samples = s.edge(e, 100);
        let inner = &samples[5..=95];
        let m = s.edge_maximum::<100>(e);
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
    let m = s.edge_maximum::<100>(Edge::X0);
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
    // 自由辺の無い組（FFSS）は隅まで含める（ADR kirchhoff-plate-0008）。x = Lx の単純支持の辺のせん断力の最大は辺の端にある
    let (lx, ly) = (2000.0, 3000.0);
    let s = solve(&plate(lx, ly, 0.2, 10.0, FFSS)).unwrap();
    for e in Edge::ALL {
        let samples = s.edge(e, 100);
        let m = s.edge_maximum::<100>(e);
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
    let along = s.edge_maximum::<100>(Edge::XLx).shear.along.as_mm();
    assert!(along < 0.05 * ly || along > 0.95 * ly, "{along}");
}

#[test]
fn a_point_is_near_a_corner_when_both_x_and_y_are_within_five_percent_of_an_end() {
    // ADR kirchhoff-plate-0008: 隅から辺長の 5 % 以内の点は、x と y のどちらも辺長の 5 % より端に寄った点。ちょうど 5 % の点は
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

#[test]
fn the_deflection_maximum_of_a_fixed_or_simply_supported_plate_is_at_the_centre() {
    // 四辺の支持が同じ板では、対称から最大は中央。値は中央の w に、位置は辺長の 1e-6 に一致する
    for supports in [FFFF, Supports::all(Support::SimplySupported)] {
        for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (5000.0, 2000.0)] {
            let s = solve(&plate(lx, ly, 0.2, 10.0, supports)).unwrap();
            let m = s.deflection_maximum();
            let centre = at(&s, lx / 2.0, ly / 2.0).w.as_mm();
            let what = format!("{lx} × {ly}");
            assert_within(m.w.as_mm(), centre, 1e-9 * centre, &what);
            assert_within(m.x.as_mm(), lx / 2.0, 1e-6 * lx, &what);
            assert_within(m.y.as_mm(), ly / 2.0, 1e-6 * ly, &what);
        }
    }
}

#[test]
fn with_one_free_edge_the_deflection_maximum_is_at_the_middle_of_that_edge() {
    // 三辺固定一辺自由。自由辺をどの辺に置いても、最大はその辺の中央。中央の w より大きい。辺長は
    // 縦横比が割り切れない値も入れる（代表の向きとの換算の丸めを見る）
    let with_free = |edge: Edge| {
        let mut supports = FFFF;
        match edge {
            Edge::X0 => supports.x0 = Support::Free,
            Edge::XLx => supports.x_lx = Support::Free,
            Edge::Y0 => supports.y0 = Support::Free,
            Edge::YLy => supports.y_ly = Support::Free,
        }
        supports
    };
    for (lx, ly, edge) in [(2000.0, 3000.0), (3100.0, 1700.0), (4000.0, 4900.0)]
        .into_iter()
        .flat_map(|(lx, ly)| Edge::ALL.map(|e| (lx, ly, e)))
    {
        let s = solve(&plate(lx, ly, 0.2, 10.0, with_free(edge))).unwrap();
        let m = s.deflection_maximum();
        let (x, y) = match edge {
            Edge::X0 => (0.0, ly / 2.0),
            Edge::XLx => (lx, ly / 2.0),
            Edge::Y0 => (lx / 2.0, 0.0),
            Edge::YLy => (lx / 2.0, ly),
        };
        let what = format!("{lx} × {ly} {edge}");
        assert_within(m.x.as_mm(), x, 1e-6 * lx, &what);
        assert_within(m.y.as_mm(), y, 1e-6 * ly, &what);
        // 辺に直交する座標は辺の上にちょうど来る（丸めで板の外や辺のすぐ内に出ない）
        let normal = match edge {
            Edge::X0 | Edge::XLx => (m.x.as_mm(), x),
            Edge::Y0 | Edge::YLy => (m.y.as_mm(), y),
        };
        assert_eq!(normal.0, normal.1, "{what}");
        assert_within(
            m.w.as_mm(),
            at(&s, x, y).w.as_mm(),
            1e-9 * m.w.as_mm(),
            &what,
        );
        assert!(m.w.as_mm() > at(&s, lx / 2.0, ly / 2.0).w.as_mm(), "{what}");
    }
}

#[test]
fn mirrored_supports_give_a_mirrored_deflection_maximum() {
    // 対称でない組（x = 0 と y = 0 が固定、x = Lx が単純支持、y = Ly が自由）と、それを x について裏返した組。
    // 最大の位置は x について裏返しに、値は同じになる
    let (lx, ly) = (3000.0, 2000.0);
    let supports = Supports {
        x0: Support::Fixed,
        y0: Support::Fixed,
        x_lx: Support::SimplySupported,
        y_ly: Support::Free,
    };
    let mirrored = Supports {
        x0: supports.x_lx,
        x_lx: supports.x0,
        ..supports
    };
    let m = solve(&plate(lx, ly, 0.2, 10.0, supports))
        .unwrap()
        .deflection_maximum();
    let n = solve(&plate(lx, ly, 0.2, 10.0, mirrored))
        .unwrap()
        .deflection_maximum();
    // 最大は自由辺の上で、単純支持の辺に寄る
    assert_within(m.y.as_mm(), ly, 1e-9 * ly, "y");
    assert!(
        m.x.as_mm() > lx / 2.0 && m.x.as_mm() < lx,
        "x = {}",
        m.x.as_mm()
    );
    assert_within(n.x.as_mm(), lx - m.x.as_mm(), 1e-6 * lx, "裏返した x");
    assert_within(n.y.as_mm(), m.y.as_mm(), 1e-6 * ly, "裏返した y");
    assert_within(n.w.as_mm(), m.w.as_mm(), 1e-9 * m.w.as_mm(), "w");
}

#[test]
fn equal_maxima_on_opposite_free_edges_give_the_one_nearer_the_origin() {
    // 対辺が自由、ほかの 2 辺が固定か単純支持。最大は両方の自由辺の中央で同じ大きさで、原点に近い側を返す
    let (lx, ly) = (2000.0, 3000.0);
    let (f, ss, r) = (Support::Fixed, Support::SimplySupported, Support::Free);
    for (supports, x, y, opposite) in [
        (
            Supports {
                x0: f,
                y0: r,
                x_lx: f,
                y_ly: r,
            },
            lx / 2.0,
            0.0,
            (lx / 2.0, ly),
        ),
        (
            Supports {
                x0: r,
                y0: f,
                x_lx: r,
                y_ly: f,
            },
            0.0,
            ly / 2.0,
            (lx, ly / 2.0),
        ),
        (
            Supports {
                x0: ss,
                y0: r,
                x_lx: ss,
                y_ly: r,
            },
            lx / 2.0,
            0.0,
            (lx / 2.0, ly),
        ),
    ] {
        let s = solve(&plate(lx, ly, 0.2, 10.0, supports)).unwrap();
        let m = s.deflection_maximum();
        assert_within(m.x.as_mm(), x, 1e-6 * lx, "x");
        assert_within(m.y.as_mm(), y, 1e-6 * ly, "y");
        assert_within(
            m.w.as_mm(),
            at(&s, opposite.0, opposite.1).w.as_mm(),
            1e-9 * m.w.as_mm(),
            "反対の自由辺の中央",
        );
    }
}
