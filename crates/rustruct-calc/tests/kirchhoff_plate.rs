//! Kirchhoff 板の計算核。入口 `solve` からだけ確かめる。期待値は理論から言える値（帯板の極限、対称、
//! 釣り合い、ν の換算）と、項数を変えたときの収束だけで、文献の数値は置かない。

mod common;
use common::assert_within;
use rustruct_calc::kirchhoff_plate::{
    Plate, PlateError, PointResponse, Solution, SupportType, solve,
};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};
use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};

/// 辺長は mm、p は kN/m²。t = 100 mm、E = 20 000 N/mm²
fn plate(lx: f64, ly: f64, nu: f64, p: f64, supports: Supports) -> Plate {
    Plate {
        lx: Length::mm(lx),
        ly: Length::mm(ly),
        thickness: Length::mm(100.0),
        young_modulus: Stress::n_per_mm2(20_000.0),
        poisson_ratio: PoissonRatio::new(nu).unwrap(),
        load: AreaLoad::kn_per_m2(p),
        supports,
    }
}

fn fixed(lx: f64, ly: f64, nu: f64, p: f64) -> Plate {
    plate(lx, ly, nu, p, Supports::all(Support::Fixed))
}

/// 曲げ剛性 D = E t³ / (12 (1 − ν²)) [N·mm]
fn rigidity(plate: &Plate) -> f64 {
    let t = plate.thickness.as_mm();
    plate.young_modulus.as_n_per_mm2() * t * t * t
        / (12.0 * (1.0 - plate.poisson_ratio.value().powi(2)))
}

fn at(s: &Solution, x: f64, y: f64) -> PointResponse {
    s.at(Length::mm(x), Length::mm(y)).expect("板の上の点")
}

#[test]
fn unstable_combinations_are_refused() {
    let rrrr = plate(1000.0, 1000.0, 0.0, 1.0, Supports::all(Support::Free));
    assert_eq!(
        solve(&rrrr).err(),
        Some(PlateError::Unstable(SupportType::RRRR))
    );
    let mut rrrs = Supports::all(Support::Free);
    rrrs.y_ly = Support::SimplySupported;
    assert_eq!(
        solve(&plate(1000.0, 1000.0, 0.0, 1.0, rrrs)).err(),
        Some(PlateError::Unstable(SupportType::RRRS))
    );
}

#[test]
fn types_without_a_solution_yet_are_unsupported() {
    let mut fffs = Supports::all(Support::Fixed);
    fffs.y_ly = Support::SimplySupported;
    assert_eq!(
        solve(&plate(1000.0, 1000.0, 0.0, 1.0, fffs)).err(),
        Some(PlateError::Unsupported(SupportType::FFFS))
    );
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
fn a_long_plate_behaves_as_a_fixed_edge_strip_across_the_short_span() {
    // Ly/Lx = 6。短辺方向の中央断面は両端が固定の梁: 中央 pL²/24、固定辺 −pL²/12、たわみ pL⁴/(384D)。
    // 長辺の影響は Ly/Lx = 6 では 1e-4 未満（収束テストと同じ項数で、比 4 で 1e-3 程度だったものが指数的に減る）。
    // 許容は帯板の値の 5e-4
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
        5e-4 * m_strip,
        "中央の Mx",
    );
    assert_within(
        edge.mx.as_nmm_per_mm(),
        -2.0 * m_strip,
        5e-4 * 2.0 * m_strip,
        "固定辺の Mx",
    );
    assert_within(
        center.my.as_nmm_per_mm(),
        0.0,
        5e-4 * m_strip,
        "中央の My（ν = 0 なら 0）",
    );
    let w_strip = q * lx.powi(4) / (384.0 * rigidity(&p));
    assert_within(center.w.as_mm(), w_strip, 5e-4 * w_strip, "中央のたわみ");
    // 固定辺のせん断力は pL/2。x = 0 の辺では Qx = +pL/2（Qx = −D w_xxx で、x = 0 の辺で w_xxx < 0）
    assert_within(
        edge.qx.as_n_per_mm(),
        q * lx / 2.0,
        5e-4 * q * lx / 2.0,
        "x = 0 の辺のせん断力",
    );
}

#[test]
fn fixed_edges_have_zero_deflection_and_zero_slope() {
    // 辺の上のたわみは級数の各項が 0 なので厳密に 0。許容は中央のたわみの 1e-12（丸め）
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

/// 辺の反力の台形則の積分 [N]
fn integrated_reaction(s: &Solution, edge: Edge, divisions: usize) -> f64 {
    let samples = s.edge(edge, divisions);
    samples
        .windows(2)
        .map(|pair| {
            let ds = pair[1].along.as_mm() - pair[0].along.as_mm();
            (pair[0].reaction.as_n_per_mm() + pair[1].reaction.as_n_per_mm()) / 2.0 * ds
        })
        .sum()
}

#[test]
fn edge_reactions_balance_the_total_load() {
    // 辺の反力の合計 − 隅反力の合計 = 荷重の合計（ADR-0004）。四辺固定では隅反力は 0。
    // 辺の反力は角の近くで級数が振れる（角から辺長の 5 % 以内で最大の 2 % 程度）ので、台形則の積分は
    // 荷重の合計の 1e-3 を許す
    for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (4000.0, 2000.0)] {
        let p = fixed(lx, ly, 0.15, 8.0);
        let s = solve(&p).unwrap();
        let total = p.load.as_n_per_mm2() * lx * ly;
        let reactions: f64 = Edge::ALL
            .iter()
            .map(|&e| integrated_reaction(&s, e, 400))
            .sum();
        let corners: f64 = Corner::ALL
            .iter()
            .map(|&c| s.corner_reaction(c).as_n())
            .sum();
        assert_within(
            reactions - corners,
            total,
            1e-3 * total,
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
fn corner_reactions_of_a_fixed_plate_are_zero() {
    let s = solve(&fixed(2000.0, 3000.0, 0.2, 10.0)).unwrap();
    for c in Corner::ALL {
        assert_eq!(s.corner_reaction(c).as_n(), 0.0, "{c:?}");
    }
}

#[test]
fn swapping_lx_and_ly_swaps_x_and_y_in_the_results() {
    // 2000×3000 の (x, y) と 3000×2000 の (y, x) は同じ板。別の級数（Lx で無次元化し、荷重の項の向きを
    // 点ごとに選ぶ）で解くので、差は打ち切り誤差。角から辺長の 5 % 以内を避けた点で、各量の中央の
    // 値（たわみ・曲げモーメント）か辺の中央の値（せん断力）の 1e-3 を許す（収束テストと同じ目安）
    let a = solve(&fixed(2000.0, 3000.0, 0.25, 10.0)).unwrap();
    let b = solve(&fixed(3000.0, 2000.0, 0.25, 10.0)).unwrap();
    let w_scale = at(&a, 1000.0, 1500.0).w.as_mm();
    let m_scale = at(&a, 0.0, 1500.0).mx.as_nmm_per_mm().abs();
    let q_scale = at(&a, 0.0, 1500.0).qx.as_n_per_mm().abs();
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
            1e-3 * w_scale,
            &format!("{what} の w"),
        );
        assert_within(
            pa.mx.as_nmm_per_mm(),
            pb.my.as_nmm_per_mm(),
            1e-3 * m_scale,
            &format!("{what} の Mx"),
        );
        assert_within(
            pa.my.as_nmm_per_mm(),
            pb.mx.as_nmm_per_mm(),
            1e-3 * m_scale,
            &format!("{what} の My"),
        );
        assert_within(
            pa.qx.as_n_per_mm(),
            pb.qy.as_n_per_mm(),
            1e-3 * q_scale,
            &format!("{what} の Qx"),
        );
        assert_within(
            pa.qy.as_n_per_mm(),
            pb.qx.as_n_per_mm(),
            1e-3 * q_scale,
            &format!("{what} の Qy"),
        );
    }
    // 辺の反力も入れ替わる
    let va = a.edge(Edge::X0, 10);
    let vb = b.edge(Edge::Y0, 10);
    for (p, q) in va.iter().zip(&vb).skip(1).take(9) {
        assert_within(
            p.reaction.as_n_per_mm(),
            q.reaction.as_n_per_mm(),
            1e-3 * q_scale,
            "x = 0 と y = 0 の反力",
        );
    }
}

#[test]
fn a_square_plate_is_symmetric_in_x_and_y_and_about_its_center() {
    // 同じ解の中の対称なので、差は丸めだけ。許容は各量の代表値の 1e-9
    let l = 2500.0;
    let s = solve(&fixed(l, l, 0.3, 10.0)).unwrap();
    let w_scale = at(&s, l / 2.0, l / 2.0).w.as_mm();
    let m_scale = at(&s, 0.0, l / 2.0).mx.as_nmm_per_mm().abs();
    let q_scale = at(&s, 0.0, l / 2.0).qx.as_n_per_mm().abs();
    for (x, y) in [
        (300.0, 900.0),
        (1250.0, 400.0),
        (2000.0, 2300.0),
        (0.0, 700.0),
    ] {
        let p = at(&s, x, y);
        let t = at(&s, y, x);
        assert_within(
            p.mx.as_nmm_per_mm(),
            t.my.as_nmm_per_mm(),
            1e-9 * m_scale,
            "Mx(x, y) = My(y, x)",
        );
        assert_within(
            p.qx.as_n_per_mm(),
            t.qy.as_n_per_mm(),
            1e-9 * q_scale,
            "Qx(x, y) = Qy(y, x)",
        );
        let m = at(&s, l - x, y);
        assert_within(
            p.w.as_mm(),
            m.w.as_mm(),
            1e-9 * w_scale,
            "w(x, y) = w(Lx − x, y)",
        );
        assert_within(
            p.mx.as_nmm_per_mm(),
            m.mx.as_nmm_per_mm(),
            1e-9 * m_scale,
            "Mx(x, y) = Mx(Lx − x, y)",
        );
        assert_within(
            p.qx.as_n_per_mm(),
            -m.qx.as_n_per_mm(),
            1e-9 * q_scale,
            "Qx(x, y) = −Qx(Lx − x, y)",
        );
    }
}

#[test]
fn poisson_ratio_scales_the_deflection_and_mixes_the_moments() {
    // 自由辺の無い型では無次元の解が ν に依らない（ADR-0005）: w ∝ 1/D ∝ 1 − ν²、Mx(ν) = Mx(0) + ν·My(0)。
    // 同じ級数の線形な組み合わせなので、差は丸めだけ（許容 1e-12）
    let (lx, ly) = (2000.0, 2800.0);
    let nu = 0.3;
    let s0 = solve(&fixed(lx, ly, 0.0, 10.0)).unwrap();
    let s1 = solve(&fixed(lx, ly, nu, 10.0)).unwrap();
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
            1e-12 * p0.w.as_mm().abs().max(1e-9),
            "w ∝ 1 − ν²",
        );
        let m_scale = p0.mx.as_nmm_per_mm().abs() + p0.my.as_nmm_per_mm().abs();
        assert_within(
            p1.mx.as_nmm_per_mm(),
            p0.mx.as_nmm_per_mm() + nu * p0.my.as_nmm_per_mm(),
            1e-12 * m_scale,
            "Mx(ν) = Mx(0) + ν·My(0)",
        );
        assert_within(
            p1.my.as_nmm_per_mm(),
            p0.my.as_nmm_per_mm() + nu * p0.mx.as_nmm_per_mm(),
            1e-12 * m_scale,
            "My(ν) = My(0) + ν·Mx(0)",
        );
        // せん断力は ν に依らない
        assert_within(
            p1.qx.as_n_per_mm(),
            p0.qx.as_n_per_mm(),
            1e-12 * p0.qx.as_n_per_mm().abs().max(1e-9),
            "Qx は ν に依らない",
        );
    }
}

/// 角から辺長の 5 % 以内（x も y も辺の近く）の点か
fn near_corner(p: &PointResponse, lx: f64, ly: f64) -> bool {
    let (x, y) = (p.x.as_mm(), p.y.as_mm());
    !(0.05 * lx..=0.95 * lx).contains(&x) && !(0.05 * ly..=0.95 * ly).contains(&y)
}

/// 点の応答から 1 つの量を取り出す
type Pick = fn(&PointResponse) -> f64;

#[test]
fn doubling_the_terms_changes_nothing_beyond_the_tolerance() {
    // ADR-0006 の決め方。縦横比 1・2・4 で、角から辺長の 5 % 以内を除いた 21×21 のグリッドと辺の 101 点で、
    // N = TERMS と 2N の差がその量の絶対値の最大の 1e-3 以下。項数 80 はこれを満たす最小の値（40・60 では
    // 比 4 のせん断力が 3.5e-3・1.3e-3）
    use rustruct_calc::kirchhoff_plate::{TERMS, solve_with_terms};
    let lx = 2000.0;
    for ratio in [1.0, 2.0, 4.0] {
        let ly = ratio * lx;
        let p = fixed(lx, ly, 0.2, 10.0);
        let a = solve_with_terms(&p, TERMS).unwrap();
        let b = solve_with_terms(&p, 2 * TERMS).unwrap();
        let (ga, gb) = (a.grid(20, 20), b.grid(20, 20));
        let pick: [(&str, Pick); 5] = [
            ("w", |p| p.w.as_mm()),
            ("Mx", |p| p.mx.as_nmm_per_mm()),
            ("My", |p| p.my.as_nmm_per_mm()),
            ("Qx", |p| p.qx.as_n_per_mm()),
            ("Qy", |p| p.qy.as_n_per_mm()),
        ];
        for (name, f) in pick {
            let scale = ga.iter().map(|p| f(p).abs()).fold(0.0, f64::max);
            for (pa, pb) in ga.iter().zip(&gb).filter(|(p, _)| !near_corner(p, lx, ly)) {
                assert_within(
                    f(pa),
                    f(pb),
                    1e-3 * scale,
                    &format!("比 {ratio} の {name} ({}, {})", pa.x.as_mm(), pa.y.as_mm()),
                );
            }
        }
        for e in Edge::ALL {
            let (ea, eb) = (a.edge(e, 100), b.edge(e, 100));
            let v_scale = ea
                .iter()
                .map(|s| s.reaction.as_n_per_mm().abs())
                .fold(0.0, f64::max);
            let q_scale = ea
                .iter()
                .map(|s| s.shear.as_n_per_mm().abs())
                .fold(0.0, f64::max);
            for (i, (sa, sb)) in ea.iter().zip(&eb).enumerate() {
                if !(5..=95).contains(&i) {
                    continue;
                }
                assert_within(
                    sa.reaction.as_n_per_mm(),
                    sb.reaction.as_n_per_mm(),
                    1e-3 * v_scale,
                    &format!("比 {ratio} の {e} の反力 {i}"),
                );
                assert_within(
                    sa.shear.as_n_per_mm(),
                    sb.shear.as_n_per_mm(),
                    1e-3 * q_scale,
                    &format!("比 {ratio} の {e} のせん断力 {i}"),
                );
                assert_within(
                    sa.moment.as_nmm_per_mm(),
                    sb.moment.as_nmm_per_mm(),
                    1e-3 * ea[50].moment.as_nmm_per_mm().abs(),
                    &format!("比 {ratio} の {e} の曲げモーメント {i}"),
                );
            }
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
    }
    // 辺の上の曲げモーメントは内部の評価と同じ値（辺の中央、打ち切りの差だけ）
    let edge = s.edge(Edge::X0, 2)[1];
    let point = at(&s, 0.0, l / 2.0);
    assert_within(
        edge.moment.as_nmm_per_mm(),
        point.mx.as_nmm_per_mm(),
        1e-3 * point.mx.as_nmm_per_mm().abs(),
        "辺の中央の Mx",
    );
    assert_within(
        edge.shear.as_n_per_mm(),
        point.qx.as_n_per_mm(),
        1e-3 * point.qx.as_n_per_mm().abs(),
        "辺の中央の Qx",
    );
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
