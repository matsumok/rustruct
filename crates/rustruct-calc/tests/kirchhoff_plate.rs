//! Kirchhoff 板の計算核の入口と、四辺固定（FFFF）の板。入口 `solve`（有限要素）からだけ確かめる。期待値は
//! 理論から言える値（帯板の極限、境界条件、釣り合い、対称、ν の換算）と、分割数を倍にしたときの変化だけ。
//! 級数解との照合は `kirchhoff_plate_series_reference.rs`

mod common;
use common::{
    assert_deviations_within_tolerance, assert_within, at, integrated_reaction, plate,
    reactions_minus_corners, rigidity,
};
use rustruct_calc::kirchhoff_plate::{
    DIVISIONS, EdgeSample, Plate, PlateError, PointResponse, Solution, SupportType, mesh_divisions,
    solve, solve_with_divisions,
};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};
use rustruct_types::quantity::{AreaLoad, Length, Stress};

const FFFF: Supports = Supports::all(Support::Fixed);
const SSSS: Supports = Supports::all(Support::SimplySupported);
/// 三辺固定一辺自由。自由辺は y = Ly
const FFFR: Supports = Supports {
    x0: Support::Fixed,
    y0: Support::Fixed,
    x_lx: Support::Fixed,
    y_ly: Support::Free,
};

fn fixed(lx: f64, ly: f64, nu: f64, p: f64) -> Plate {
    plate(lx, ly, nu, p, FFFF)
}

#[test]
fn unstable_types_are_refused_and_the_other_nineteen_solve() {
    for ty in SupportType::ALL {
        let p = plate(2000.0, 3000.0, 0.2, 10.0, ty.representative());
        let result = solve(&p);
        match ty {
            SupportType::RRRR | SupportType::RRRS => {
                assert_eq!(result.err(), Some(PlateError::Unstable(ty)));
            }
            _ => {
                let s = result.unwrap_or_else(|e| panic!("{}: {e}", ty.symbol()));
                assert_eq!(s.support_type(), ty);
                let centre = at(&s, 1000.0, 1500.0);
                assert!(
                    centre.w.as_mm().is_finite() && centre.w.as_mm() > 0.0,
                    "{}",
                    ty.symbol()
                );
                // 釣り合いは型によらず丸め誤差で成り立つ
                let total = p.load.as_n_per_mm2() * 2000.0 * 3000.0;
                assert_within(
                    reactions_minus_corners(&s),
                    total,
                    1e-7 * total,
                    &format!("{} の釣り合い", ty.symbol()),
                );
            }
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
fn a_long_plate_behaves_as_a_fixed_edge_strip_across_the_short_span() {
    // Ly/Lx = 6。短辺方向の中央断面は両端が固定の梁: 中央 pL²/24、固定辺 −pL²/12、たわみ pL⁴/(384D)、
    // 固定辺のせん断力 pL/2。長辺の影響は Ly/Lx = 6 では 1e-4 未満（短辺の幅ごとに指数的に減る）。
    // 許容は帯板の値の、たわみと曲げモーメントで 5e-3、せん断力で 2e-2（有限要素の離散化の誤差。ADR-0002）
    let lx = 2000.0;
    let p = fixed(lx, 6.0 * lx, 0.0, 10.0);
    let s = solve(&p).unwrap();
    let q = p.load.as_n_per_mm2();
    let center = at(&s, lx / 2.0, 3.0 * lx);
    let edge = at(&s, 0.0, 3.0 * lx);
    let m_strip = q * lx * lx / 24.0;
    assert_within(
        center.mx.as_nmm_per_mm(),
        m_strip,
        5e-3 * m_strip,
        "中央の Mx",
    );
    assert_within(
        edge.mx.as_nmm_per_mm(),
        -2.0 * m_strip,
        5e-3 * 2.0 * m_strip,
        "固定辺の Mx",
    );
    assert_within(
        center.my.as_nmm_per_mm(),
        0.0,
        5e-3 * m_strip,
        "中央の My（ν = 0 なら 0）",
    );
    let w_strip = q * lx.powi(4) / (384.0 * rigidity(&p));
    assert_within(center.w.as_mm(), w_strip, 5e-3 * w_strip, "中央のたわみ");
    // 固定辺のせん断力は pL/2。x = 0 の辺では Qx = +pL/2（Qx = −D w_xxx で、x = 0 の辺で w_xxx < 0）
    assert_within(
        edge.qx.as_n_per_mm(),
        q * lx / 2.0,
        2e-2 * q * lx / 2.0,
        "x = 0 の辺のせん断力",
    );
    // 辺の中央ではねじれが無いので、反力はせん断力と同じ pL/2
    let mid = s.edge(Edge::X0, 2)[1];
    assert_within(
        mid.reaction.as_n_per_mm(),
        q * lx / 2.0,
        2e-2 * q * lx / 2.0,
        "x = 0 の辺の反力",
    );
}

#[test]
fn fixed_edges_have_zero_deflection_and_zero_slope() {
    // 固定辺の節点は w と傾きをすべて拘束するので、辺の上のたわみは形状関数の値として厳密に 0。
    // 許容は中央のたわみの 1e-12（丸め）
    let lx = 3000.0;
    let s = solve(&fixed(lx, 2000.0, 0.2, 10.0)).unwrap();
    let w_center = at(&s, lx / 2.0, 1000.0).w.as_mm();
    for (x, y) in [(0.0, 700.0), (lx, 1300.0), (1100.0, 0.0), (1900.0, 2000.0)] {
        assert_within(
            at(&s, x, y).w.as_mm(),
            0.0,
            1e-12 * w_center,
            &format!("({x}, {y}) のたわみ"),
        );
    }
    // 固定辺では傾きも 0 なので、辺からの距離 h のたわみは h² に比例する: w(h)/w(2h) → 1/4。
    // 単純支持なら h に比例し 1/2 になる。h = 辺長の 1/200 で、h³ の項による比のずれは 1/200 程度なので許容 1e-2
    let h = lx / 200.0;
    let ratio_x = at(&s, h, 1000.0).w.as_mm() / at(&s, 2.0 * h, 1000.0).w.as_mm();
    assert_within(ratio_x, 0.25, 1e-2, "x = 0 の辺の近くの w(h)/w(2h)");
    let h = 2000.0 / 200.0;
    let ratio_y = at(&s, 1500.0, 2000.0 - h).w.as_mm() / at(&s, 1500.0, 2000.0 - 2.0 * h).w.as_mm();
    assert_within(ratio_y, 0.25, 1e-2, "y = Ly の辺の近くの w(h)/w(2h)");
}

#[test]
fn edge_reactions_balance_the_total_load() {
    // 辺の反力の合計 − 隅反力の合計 = 荷重の合計（ADR-0004）。辺の反力の合計は節点の反力の和そのもので、
    // 丸め誤差（連立の大きさで 1e-10 の桁。許容 1e-7）で一致する。四辺固定では隅反力は 0
    for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (4000.0, 2000.0)] {
        let p = fixed(lx, ly, 0.15, 8.0);
        let s = solve(&p).unwrap();
        let total = p.load.as_n_per_mm2() * lx * ly;
        assert_within(
            reactions_minus_corners(&s),
            total,
            1e-7 * total,
            &format!("{lx}×{ly} の釣り合い"),
        );
        // 反力は支える向き（正）。辺の中央で見る
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
fn corner_reactions_of_a_fixed_plate_are_zero() {
    let s = solve(&fixed(2000.0, 3000.0, 0.2, 10.0)).unwrap();
    for c in Corner::ALL {
        assert_eq!(s.corner_reaction(c).as_n(), 0.0, "{c:?}");
    }
}

/// 丸め誤差の比較の分母。たわみ・曲げモーメント・せん断力それぞれの、グリッドでの絶対値の最大
struct Scale {
    w: f64,
    m: f64,
    q: f64,
}

impl Scale {
    fn of(s: &Solution) -> Self {
        let g = s.grid(10, 10);
        let max =
            |f: &dyn Fn(&PointResponse) -> f64| g.iter().map(|p| f(p).abs()).fold(0.0, f64::max);
        Scale {
            w: max(&|p| p.w.as_mm()),
            m: max(&|p| p.mx.as_nmm_per_mm()).max(max(&|p| p.my.as_nmm_per_mm())),
            q: max(&|p| p.qx.as_n_per_mm()).max(max(&|p| p.qy.as_n_per_mm())),
        }
    }
}

#[test]
fn swapping_lx_and_ly_swaps_x_and_y_in_the_results() {
    // 2000×3000 の (x, y) と 3000×2000 の (y, x) は同じ板。要素の分割も入れ替わるだけなので、差は丸め誤差
    let a = solve(&fixed(2000.0, 3000.0, 0.25, 10.0)).unwrap();
    let b = solve(&fixed(3000.0, 2000.0, 0.25, 10.0)).unwrap();
    let scale = Scale::of(&a);
    for (x, y) in [
        (1000.0, 1500.0),
        (300.0, 1500.0),
        (1000.0, 200.0),
        (1700.0, 2600.0),
        (0.0, 1500.0),
        (1000.0, 3000.0),
    ] {
        let pa = at(&a, x, y);
        let pb = at(&b, y, x);
        let what = format!("({x}, {y})");
        assert_within(
            pa.w.as_mm(),
            pb.w.as_mm(),
            1e-9 * scale.w,
            &format!("{what} の w"),
        );
        assert_within(
            pa.mx.as_nmm_per_mm(),
            pb.my.as_nmm_per_mm(),
            1e-9 * scale.m,
            &format!("{what} の Mx"),
        );
        assert_within(
            pa.my.as_nmm_per_mm(),
            pb.mx.as_nmm_per_mm(),
            1e-9 * scale.m,
            &format!("{what} の My"),
        );
        assert_within(
            pa.qx.as_n_per_mm(),
            pb.qy.as_n_per_mm(),
            1e-9 * scale.q,
            &format!("{what} の Qx"),
        );
        assert_within(
            pa.qy.as_n_per_mm(),
            pb.qx.as_n_per_mm(),
            1e-9 * scale.q,
            &format!("{what} の Qy"),
        );
    }
    // 辺の反力も入れ替わる
    for (ea, eb) in [(Edge::X0, Edge::Y0), (Edge::YLy, Edge::XLx)] {
        for (sa, sb) in a.edge(ea, 10).iter().zip(b.edge(eb, 10)) {
            assert_within(
                sa.reaction.as_n_per_mm(),
                sb.reaction.as_n_per_mm(),
                1e-9 * scale.q,
                &format!("{ea} の反力"),
            );
        }
    }
}

#[test]
fn a_square_plate_is_symmetric_under_the_symmetries_of_the_square() {
    // 四辺固定の正方形は、x と y の入れ替え、x の裏返し、y の裏返しで同じ板。要素の分割も対称なので、
    // 差は丸め誤差。せん断力は裏返す軸の向きで符号が変わる
    let l = 2500.0;
    let s = solve(&fixed(l, l, 0.3, 10.0)).unwrap();
    let scale = Scale::of(&s);
    let close = |p: f64, q: f64, what: &str| {
        let unit = if what.ends_with('w') {
            scale.w
        } else if what.contains('M') || what.contains("モーメント") {
            scale.m
        } else {
            scale.q
        };
        assert_within(p, q, 1e-9 * unit, what);
    };
    for (x, y) in [
        (300.0, 900.0),
        (1250.0, 400.0),
        (2000.0, 2300.0),
        (0.0, 700.0),
    ] {
        let r = at(&s, x, y);
        let t = at(&s, y, x);
        close(r.w.as_mm(), t.w.as_mm(), "入れ替えの w");
        close(r.mx.as_nmm_per_mm(), t.my.as_nmm_per_mm(), "入れ替えの Mx");
        close(r.qx.as_n_per_mm(), t.qy.as_n_per_mm(), "入れ替えの Qx");
        let fx = at(&s, l - x, y);
        close(r.w.as_mm(), fx.w.as_mm(), "x の裏返しの w");
        close(
            r.mx.as_nmm_per_mm(),
            fx.mx.as_nmm_per_mm(),
            "x の裏返しの Mx",
        );
        close(
            r.my.as_nmm_per_mm(),
            fx.my.as_nmm_per_mm(),
            "x の裏返しの My",
        );
        close(r.qx.as_n_per_mm(), -fx.qx.as_n_per_mm(), "x の裏返しの Qx");
        close(r.qy.as_n_per_mm(), fx.qy.as_n_per_mm(), "x の裏返しの Qy");
        let fy = at(&s, x, l - y);
        close(r.w.as_mm(), fy.w.as_mm(), "y の裏返しの w");
        close(r.qy.as_n_per_mm(), -fy.qy.as_n_per_mm(), "y の裏返しの Qy");
    }
    let edges: Vec<Vec<EdgeSample>> = Edge::ALL.iter().map(|&e| s.edge(e, 10)).collect();
    for samples in &edges[1..] {
        for (a, b) in edges[0].iter().zip(samples) {
            close(
                a.reaction.as_n_per_mm(),
                b.reaction.as_n_per_mm(),
                "辺の反力",
            );
            close(
                a.moment.as_nmm_per_mm(),
                b.moment.as_nmm_per_mm(),
                "辺の曲げモーメント",
            );
        }
    }
}

#[test]
fn poisson_ratio_scales_the_deflection_and_mixes_the_moments() {
    // 自由辺の無い型では無次元の解が ν に依らない（ADR-0005）: w ∝ 1/D ∝ 1 − ν²、Mx(ν) = Mx(0) + ν·My(0)。
    // 有限要素でも同じ。ひずみエネルギーの ν の項 2ν (w_xx w_yy − w_xy²) は、要素の間で w と傾きが連続なので
    // 板全体では辺の上の値だけで決まり、固定辺と単純支持の辺では 0 になる。差は連立の丸め（許容 1e-9）
    let (lx, ly) = (2000.0, 2800.0);
    let nu = 0.3;
    let s0 = solve(&fixed(lx, ly, 0.0, 10.0)).unwrap();
    let s1 = solve(&fixed(lx, ly, nu, 10.0)).unwrap();
    let scale = Scale::of(&s0);
    for (x, y) in [
        (1000.0, 1400.0),
        (0.0, 1400.0),
        (600.0, 2800.0),
        (1500.0, 500.0),
    ] {
        let p0 = at(&s0, x, y);
        let p1 = at(&s1, x, y);
        assert_within(
            p1.w.as_mm(),
            (1.0 - nu * nu) * p0.w.as_mm(),
            1e-9 * scale.w,
            "w ∝ 1 − ν²",
        );
        assert_within(
            p1.mx.as_nmm_per_mm(),
            p0.mx.as_nmm_per_mm() + nu * p0.my.as_nmm_per_mm(),
            1e-9 * scale.m,
            "Mx(ν) = Mx(0) + ν·My(0)",
        );
        assert_within(
            p1.my.as_nmm_per_mm(),
            p0.my.as_nmm_per_mm() + nu * p0.mx.as_nmm_per_mm(),
            1e-9 * scale.m,
            "My(ν) = My(0) + ν·Mx(0)",
        );
        // せん断力は ν に依らない
        assert_within(
            p1.qx.as_n_per_mm(),
            p0.qx.as_n_per_mm(),
            1e-9 * scale.q,
            "Qx は ν に依らない",
        );
    }
}

#[test]
fn doubling_the_divisions_changes_nothing_beyond_the_tolerance() {
    // ADR-0006 の決め方。縦横比 1・2・4、ν = 0.2 で、短い辺の分割数を DIVISIONS から倍にしたときの差が、
    // 級数解との照合と同じ許容（隅から辺長の 5 % を除いて、w・M は 0.5 %、Q・V は 2 %）に収まる。
    // 四辺固定・四辺単純支持に、級数解の無い三辺固定一辺自由を加える
    let lx = 2000.0;
    for (symbol, supports) in [("SSSS", SSSS), ("FFFF", FFFF), ("FFFR", FFFR)] {
        for ratio in [1.0, 2.0, 4.0] {
            let p = plate(lx, ratio * lx, 0.2, 10.0, supports);
            let coarse = solve_with_divisions(&p, DIVISIONS).unwrap();
            let fine = solve_with_divisions(&p, 2 * DIVISIONS).unwrap();
            assert_deviations_within_tolerance(&fine, &coarse, &format!("{symbol} 比 {ratio}"));
        }
    }
}

#[test]
fn the_edge_maximum_of_a_square_plate_is_at_the_middle_of_the_edge() {
    let l = 2000.0;
    let s = solve(&fixed(l, l, 0.0, 10.0)).unwrap();
    for e in Edge::ALL {
        let m = s.edge_maximum(e, 100);
        let mid = s.edge(e, 100)[50];
        assert_eq!(m.reaction.along.as_mm(), l / 2.0, "{e} の反力の最大の位置");
        assert_eq!(m.reaction.value, mid.reaction, "{e} の反力の最大");
        assert_eq!(m.shear.along.as_mm(), l / 2.0, "{e} のせん断力の最大の位置");
        assert_eq!(m.shear.value, mid.shear, "{e} のせん断力の最大");
        assert_eq!(
            m.moment.along.as_mm(),
            l / 2.0,
            "{e} の曲げモーメントの最大の位置"
        );
        assert_eq!(m.moment.value, mid.moment, "{e} の曲げモーメントの最大");
    }
    // 辺の上の曲げモーメントとせん断力は、内部の評価と同じ導関数から求めるので同じ値
    let edge = s.edge(Edge::X0, 2)[1];
    let point = at(&s, 0.0, l / 2.0);
    assert_eq!(edge.moment, point.mx, "辺の中央の Mx");
    assert_eq!(edge.shear, point.qx, "辺の中央の Qx");
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
