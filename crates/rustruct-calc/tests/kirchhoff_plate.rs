//! Kirchhoff 板の計算核の入口（不安定な型と不正な入力を弾く、グリッドと辺の評価の並び）と、四辺固定
//! （FFFF）の板の辺の値。入口 `solve`（有限要素）からだけ確かめる。19 型すべてに共通の照合（境界条件、釣り合い、
//! 対称、分割数を倍にしたときの変化、帯板の極限、ν の換算）は `kirchhoff_plate_all_types.rs`、級数解との照合は
//! `kirchhoff_plate_series_reference.rs`

mod common;
use common::{assert_within, at, integrated_reaction, plate};
use rustruct_calc::kirchhoff_plate::{
    EdgeSample, Maxima, Maximum, Plate, PlateError, PointResponse, Position, SupportType, solve,
};
use rustruct_types::kirchhoff_plate::{Edge, PlateMaterial, Support, Supports};
use rustruct_types::quantity::{AreaLoad, ForcePerLength, Length, MomentPerLength, Stress};

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
        Some(PlateError::InvalidInput(vec![InvalidInput::Lx]))
    );
    assert_eq!(
        bad(|p| p.ly = Length::mm(-1.0)),
        Some(PlateError::InvalidInput(vec![InvalidInput::Ly]))
    );
    assert_eq!(
        bad(|p| p.thickness = Length::mm(f64::NAN)),
        Some(PlateError::InvalidInput(vec![InvalidInput::Thickness]))
    );
    assert_eq!(
        bad(|p| p.material = PlateMaterial::Direct {
            young_modulus: Stress::n_per_mm2(0.0),
            poisson_ratio: p.material.poisson_ratio(),
        }),
        Some(PlateError::InvalidInput(vec![InvalidInput::YoungModulus]))
    );
    assert_eq!(
        bad(|p| p.load = AreaLoad::n_per_mm2(f64::INFINITY)),
        Some(PlateError::InvalidInput(vec![InvalidInput::Load]))
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

/// 不正な欄が 2 つ以上あれば、最初の 1 つでなく全部を欄の順に返す（ページは読めない欄を全部いっぺんに出す）
#[test]
fn every_invalid_input_is_returned_in_the_order_of_the_fields() {
    use rustruct_calc::kirchhoff_plate::InvalidInput;
    let mut p = fixed(1000.0, 1000.0, 0.0, 1.0);
    p.load = AreaLoad::n_per_mm2(f64::NAN);
    p.thickness = Length::mm(-150.0);
    p.lx = Length::mm(0.0);
    assert_eq!(
        solve(&p).err(),
        Some(PlateError::InvalidInput(vec![
            InvalidInput::Lx,
            InvalidInput::Thickness,
            InvalidInput::Load,
        ]))
    );
}

/// 理由の文言は欄の下に出すもの。欄の名前は含めない（ページが欄に付ける）
#[test]
fn an_invalid_input_reads_as_what_to_enter() {
    use rustruct_calc::kirchhoff_plate::InvalidInput;
    for reason in [
        InvalidInput::Lx,
        InvalidInput::Ly,
        InvalidInput::Thickness,
        InvalidInput::YoungModulus,
    ] {
        assert_eq!(reason.to_string(), "0 より大きい値を入れる", "{reason:?}");
    }
    assert_eq!(InvalidInput::Load.to_string(), "数値で入れる");
}

/// 最大値の位置 (x, y) [mm]。最大値が無ければ失敗
fn position<T>(m: &Option<Maximum<T>>, what: &str) -> (f64, f64) {
    let p = m
        .as_ref()
        .unwrap_or_else(|| panic!("{what}: 位置が無い"))
        .position;
    (p.x.as_mm(), p.y.as_mm())
}

/// 曲げモーメントの最大値の値 [N·mm/mm]。最大値が無ければ 0
fn moment_value(m: &Option<Maximum<MomentPerLength>>) -> f64 {
    m.map_or(0.0, |m| m.value.as_nmm_per_mm())
}

/// せん断力の最大値の値 [N/mm]。最大値が無ければ 0
fn shear_value(m: &Option<Maximum<ForcePerLength>>) -> f64 {
    m.map_or(0.0, |m| m.value.as_n_per_mm())
}

#[test]
fn the_bottom_tension_maxima_of_a_fixed_plate_are_at_the_centre() {
    // 四辺固定。下側引張の最大が板の中央の値に一致し（1e-9）、位置が中央（辺長の 1e-6）。縦横比 1.5 の長い辺の
    // 向きの My は、中央でなく y = Ly/2 について対称な 2 点が最大で（級数解でも同じ）、原点に近い側を返す
    for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0)] {
        let s = solve(&plate(lx, ly, 0.2, 10.0, FFFF)).unwrap();
        let m = s.maxima();
        let centre = at(&s, lx / 2.0, ly / 2.0);
        let what = format!("{lx} × {ly}");
        let mut at_centre = vec![("Mx", m.mx_bottom, centre.mx.as_nmm_per_mm())];
        if lx == ly {
            at_centre.push(("My", m.my_bottom, centre.my.as_nmm_per_mm()));
        } else {
            let (x, y) = position(&m.my_bottom, &what);
            let value = moment_value(&m.my_bottom);
            assert_within(x, lx / 2.0, 1e-6 * lx, &what);
            assert!(y < ly / 2.0, "{what}: y = {y}");
            assert!(value > centre.my.as_nmm_per_mm(), "{what}");
            assert_within(
                at(&s, x, ly - y).my.as_nmm_per_mm(),
                value,
                1e-9 * value,
                &format!("{what} の y について裏返した点"),
            );
        }
        for (name, maximum, expected) in at_centre {
            let what = format!("{what} の {name}");
            let (x, y) = position(&maximum, &what);
            assert_within(moment_value(&maximum), expected, 1e-9 * expected, &what);
            assert_within(x, lx / 2.0, 1e-6 * lx, &what);
            assert_within(y, ly / 2.0, 1e-6 * ly, &what);
        }
    }
}

#[test]
fn the_top_tension_and_shear_maxima_of_a_fixed_plate_are_the_edge_maxima() {
    // 四辺固定。上側引張の Mx と Qx の最大は x = 0 の辺の、My と Qy は y = 0 の辺の辺の最大値に一致し、位置は
    // その辺の中央（対辺の同じ大きさの最大より原点に近い）。辺に直交する座標はちょうど辺の上
    for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0)] {
        let s = solve(&plate(lx, ly, 0.2, 10.0, FFFF)).unwrap();
        let m = s.maxima();
        let (x0, y0) = (
            s.edge_maximum::<100>(Edge::X0),
            s.edge_maximum::<100>(Edge::Y0),
        );
        for (name, value, found, expected, middle) in [
            (
                "Mx",
                moment_value(&m.mx_top),
                position(&m.mx_top, "Mx"),
                x0.moment.value.as_nmm_per_mm(),
                (0.0, ly / 2.0),
            ),
            (
                "My",
                moment_value(&m.my_top),
                position(&m.my_top, "My"),
                y0.moment.value.as_nmm_per_mm(),
                (lx / 2.0, 0.0),
            ),
            (
                "Qx",
                shear_value(&m.qx),
                position(&m.qx, "Qx"),
                x0.shear.value.as_n_per_mm(),
                (0.0, ly / 2.0),
            ),
            (
                "Qy",
                shear_value(&m.qy),
                position(&m.qy, "Qy"),
                y0.shear.value.as_n_per_mm(),
                (lx / 2.0, 0.0),
            ),
        ] {
            let what = format!("{lx} × {ly} の {name}");
            assert_within(value, expected, 1e-9 * expected.abs(), &what);
            assert_within(found.0, middle.0, 1e-6 * lx, &what);
            assert_within(found.1, middle.1, 1e-6 * ly, &what);
            if middle.0 == 0.0 {
                assert_eq!(found.0, 0.0, "{what}");
            } else {
                assert_eq!(found.1, 0.0, "{what}");
            }
        }
    }
}

#[test]
fn a_simply_supported_plate_shows_its_tiny_top_tension_as_it_is() {
    // 四辺単純支持。有限要素の曲げモーメントは辺の上でちょうど 0 にならず、上側引張の最大はその小さな負の値と
    // 位置になる（0 に丸めない。ADR kirchhoff-plate-0010）。大きさは M の許容（下側引張の最大の 0.5 %、
    // ADR kirchhoff-plate-0002）より小さい。下側引張は中央の値以上（長い辺の向きの曲げモーメントは、縦横比と ν
    // によって中央の外が最大になる。級数解でも同じ）
    for nu in [0.0, 0.2] {
        for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (5000.0, 2000.0)] {
            let s = solve(&plate(
                lx,
                ly,
                nu,
                10.0,
                Supports::all(Support::SimplySupported),
            ))
            .unwrap();
            let m = s.maxima();
            let centre = at(&s, lx / 2.0, ly / 2.0);
            let what = format!("ν = {nu} {lx} × {ly}");
            for (name, top, bottom) in
                [("Mx", m.mx_top, m.mx_bottom), ("My", m.my_top, m.my_bottom)]
            {
                let what = format!("{what} の {name}");
                position(&top, &what);
                let (top, bottom) = (moment_value(&top), moment_value(&bottom));
                assert!(top < 0.0 && -top < 5e-3 * bottom, "{what}: {top}");
            }
            for (name, bottom, centre) in [
                ("Mx", m.mx_bottom, centre.mx.as_nmm_per_mm()),
                ("My", m.my_bottom, centre.my.as_nmm_per_mm()),
            ] {
                let what = format!("{what} の {name}");
                position(&bottom, &what);
                assert!(moment_value(&bottom) >= centre * (1.0 - 1e-9), "{what}");
            }
        }
    }
}

#[test]
fn with_a_free_edge_the_shear_maxima_avoid_the_corners() {
    // 三辺固定一辺自由。固定辺と自由辺の隅で発散するせん断力の最大は、隅から辺長の 5 % 以内に無い
    // （ADR kirchhoff-plate-0008）。自由辺をどの辺に置いても
    for edge in Edge::ALL {
        let mut supports = FFFF;
        match edge {
            Edge::X0 => supports.x0 = Support::Free,
            Edge::XLx => supports.x_lx = Support::Free,
            Edge::Y0 => supports.y0 = Support::Free,
            Edge::YLy => supports.y_ly = Support::Free,
        }
        let s = solve(&plate(2000.0, 3000.0, 0.2, 10.0, supports)).unwrap();
        let m = s.maxima();
        for (name, q) in [("Qx", m.qx), ("Qy", m.qy)] {
            let what = format!("{edge} が自由の {name}");
            let (x, y) = position(&q, &what);
            assert!(
                !s.near_corner(Length::mm(x), Length::mm(y)),
                "{what}: ({x}, {y})"
            );
        }
    }
}

/// 最大値の名前・値（内部の単位）・位置 (x, y) [mm]
type Found = (&'static str, f64, Option<(f64, f64)>);

/// 最大値の 6 つ（曲げモーメントの向きごととせん断力）
fn bending_and_shear(m: &Maxima) -> [Found; 6] {
    let at = |p: Position| (p.x.as_mm(), p.y.as_mm());
    let moment =
        |v: &Option<Maximum<MomentPerLength>>| (moment_value(v), v.map(|v| at(v.position)));
    let shear = |v: &Option<Maximum<ForcePerLength>>| (shear_value(v), v.map(|v| at(v.position)));
    let named = |name, (value, at): (f64, Option<(f64, f64)>)| -> Found { (name, value, at) };
    [
        named("Mx の下側引張", moment(&m.mx_bottom)),
        named("Mx の上側引張", moment(&m.mx_top)),
        named("My の下側引張", moment(&m.my_bottom)),
        named("My の上側引張", moment(&m.my_top)),
        named("Qx", shear(&m.qx)),
        named("Qy", shear(&m.qy)),
    ]
}

#[test]
fn mirrored_supports_give_mirrored_maxima() {
    // 対称でない組（x = 0 と y = 0 が固定、x = Lx が単純支持、y = Ly が自由）と、それを x について裏返した組。
    // 各値の位置は x について裏返しに、値は同じになる（Qx は x の向きが逆になるので符号が変わる）
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
    let m = solve(&plate(lx, ly, 0.2, 10.0, supports)).unwrap().maxima();
    let n = solve(&plate(lx, ly, 0.2, 10.0, mirrored)).unwrap().maxima();
    for ((name, a, at_a), (_, b, at_b)) in
        bending_and_shear(&m).into_iter().zip(bending_and_shear(&n))
    {
        let (pa, pb) = (
            at_a.unwrap_or_else(|| panic!("{name}: 位置が無い")),
            at_b.unwrap_or_else(|| panic!("裏返した {name}: 位置が無い")),
        );
        let sign = if name == "Qx" { -1.0 } else { 1.0 };
        assert_within(b, sign * a, 1e-9 * a.abs(), name);
        assert_within(pb.0, lx - pa.0, 1e-6 * lx, &format!("{name} の裏返した x"));
        assert_within(pb.1, pa.1, 1e-6 * ly, &format!("{name} の裏返した y"));
    }
}

#[test]
fn no_point_of_the_plate_exceeds_the_maxima() {
    // 19 型すべてで、内部のグリッドの点と辺の評価点のどれも、向きごとの最大値を超えない（取りこぼさない）。
    // 辺の評価点は辺に直交する方向の曲げモーメントとせん断力を見る。自由辺のある組のせん断力は、隅から辺長の
    // 5 % 以内の点を除く（ADR kirchhoff-plate-0008）。超える幅は丸めの 1e-9（その量の最大に対して）
    for ty in SupportType::ALL.into_iter().filter(|ty| ty.is_stable()) {
        let s = solve(&plate(2000.0, 3000.0, 0.2, 10.0, ty.representative())).unwrap();
        let m = s.maxima();
        let free = s.plate().supports.has_free_edge();
        let (lx, ly) = (2000.0, 3000.0);
        let interior: Vec<PointResponse> = s
            .grid(20, 20)
            .into_iter()
            .filter(|p| {
                let (x, y) = (p.x.as_mm(), p.y.as_mm());
                x > 0.0 && x < lx && y > 0.0 && y < ly
            })
            .collect();
        let within = |value: f64, maximum: f64, what: &str| {
            assert!(
                value <= maximum + 1e-9 * maximum.abs().max(1.0),
                "{} の {what}: {value} が最大 {maximum} を超える",
                ty.symbol()
            );
        };
        for p in &interior {
            within(
                p.mx.as_nmm_per_mm(),
                moment_value(&m.mx_bottom),
                "Mx の下側引張",
            );
            within(
                -p.mx.as_nmm_per_mm(),
                -moment_value(&m.mx_top),
                "Mx の上側引張",
            );
            within(
                p.my.as_nmm_per_mm(),
                moment_value(&m.my_bottom),
                "My の下側引張",
            );
            within(
                -p.my.as_nmm_per_mm(),
                -moment_value(&m.my_top),
                "My の上側引張",
            );
            if !(free && s.near_corner(p.x, p.y)) {
                within(p.qx.as_n_per_mm().abs(), shear_value(&m.qx).abs(), "|Qx|");
                within(p.qy.as_n_per_mm().abs(), shear_value(&m.qy).abs(), "|Qy|");
            }
        }
        for edge in Edge::ALL {
            let e = s.edge_maximum::<100>(edge);
            let (bottom, top, shear) = match edge {
                Edge::X0 | Edge::XLx => (m.mx_bottom, m.mx_top, m.qx),
                Edge::Y0 | Edge::YLy => (m.my_bottom, m.my_top, m.qy),
            };
            let moment = e.moment.value.as_nmm_per_mm();
            within(moment, moment_value(&bottom), &format!("{edge} の下側引張"));
            within(-moment, -moment_value(&top), &format!("{edge} の上側引張"));
            within(
                e.shear.value.as_n_per_mm().abs(),
                shear_value(&shear).abs(),
                &format!("{edge} のせん断力"),
            );
        }
    }
}

#[test]
fn the_deflection_maximum_is_the_maximum_w_and_an_unloaded_plate_has_no_maxima() {
    // 最大値の w は最大たわみそのもの。荷重が 0 の板では、どの量も値 0・位置無し
    let s = solve(&plate(2000.0, 3000.0, 0.2, 10.0, FFSS)).unwrap();
    let (m, d) = (s.maxima(), s.deflection_maximum());
    assert_eq!(m.w.map(|w| w.value), Some(d.w));
    assert_eq!(position(&m.w, "w"), (d.x.as_mm(), d.y.as_mm()));
    let m = solve(&plate(2000.0, 3000.0, 0.2, 0.0, FFSS))
        .unwrap()
        .maxima();
    assert_eq!(m.w, None);
    for (name, value, at) in bending_and_shear(&m) {
        assert_eq!(value, 0.0, "{name}");
        assert_eq!(at, None, "{name}");
    }
}
