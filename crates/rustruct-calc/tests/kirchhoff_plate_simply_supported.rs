//! 四辺単純支持（SSSS）の Kirchhoff 板。入口 `solve`（有限要素）からだけ確かめる。期待値は理論から言える値
//! （帯板の極限、境界条件、釣り合い、対称、ν の換算）と、独立な解（Navier の二重正弦級数）だけ。
//! 級数解との照合は `kirchhoff_plate_series_reference.rs`

mod common;
use common::{assert_within, at, navier, plate, reactions_minus_corners, rigidity};
use rustruct_calc::kirchhoff_plate::{
    EdgeSample, Plate, PointResponse, Solution, SupportType, solve,
};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};

fn simply_supported(lx: f64, ly: f64, nu: f64, p: f64) -> Plate {
    plate(lx, ly, nu, p, Supports::all(Support::SimplySupported))
}

#[test]
fn four_simply_supported_edges_are_the_ssss_type() {
    let s = solve(&simply_supported(2000.0, 3000.0, 0.0, 10.0)).unwrap();
    assert_eq!(s.support_type(), SupportType::SSSS);
}

#[test]
fn the_interior_and_the_corner_reactions_match_the_navier_double_series() {
    // Navier の二重級数は有限要素と独立な解。m・n を 799 まで（各 400 項）取ると、Navier の打ち切りの差は
    // たわみで相対 1e-12、曲げモーメントとせん断力で各量の最大の 1e-5 未満なので、差は有限要素の離散化の誤差。
    // 許容はたわみと曲げモーメントが各量の代表値の 1e-3、せん断力と隅反力が 1e-2（級数解との照合の最大は
    // w 3e-6、M 9e-5、Q 2.2e-3、隅反力 2e-5。ADR-0002）
    let (lx, ly) = (2000.0, 3000.0);
    let p = simply_supported(lx, ly, 0.2, 10.0);
    let s = solve(&p).unwrap();
    let centre = navier(&p, lx / 2.0, ly / 2.0, 799);
    let q_scale = navier(&p, 0.0, ly / 2.0, 799).qx.abs();
    for (x, y) in [
        (1000.0, 1500.0),
        (300.0, 700.0),
        (1700.0, 2600.0),
        (500.0, 1500.0),
        (1000.0, 200.0),
    ] {
        let ours = at(&s, x, y);
        let n = navier(&p, x, y, 799);
        let what = format!("({x}, {y})");
        assert_within(
            ours.w.as_mm(),
            n.w,
            1e-3 * centre.w,
            &format!("{what} の w"),
        );
        assert_within(
            ours.mx.as_nmm_per_mm(),
            n.mx,
            1e-3 * centre.mx,
            &format!("{what} の Mx"),
        );
        assert_within(
            ours.my.as_nmm_per_mm(),
            n.my,
            1e-3 * centre.mx,
            &format!("{what} の My"),
        );
        assert_within(
            ours.qx.as_n_per_mm(),
            n.qx,
            1e-2 * q_scale,
            &format!("{what} の Qx"),
        );
        assert_within(
            ours.qy.as_n_per_mm(),
            n.qy,
            1e-2 * q_scale,
            &format!("{what} の Qy"),
        );
    }
    // 隅反力 R = 2 D (1 − ν) |w_xy|（隅）。隅の w_xy は Navier の各項が 1/(m² + n²)² で減るので速く収束する
    let expected = 2.0 * rigidity(&p) * (1.0 - 0.2) * navier(&p, 0.0, 0.0, 799).w_xy;
    for c in Corner::ALL {
        assert_within(
            s.corner_reaction(c).as_n(),
            expected,
            1e-2 * expected,
            &format!("{c:?} の隅反力"),
        );
    }
    // 単純支持の隅では板が浮き上がろうとし、支持が押さえる（正）
    assert!(expected > 0.0);
}

#[test]
fn a_long_plate_behaves_as_a_simply_supported_strip_across_the_short_span() {
    // Ly/Lx = 8。短辺方向の中央断面は単純支持の梁: 中央 pL²/8、たわみ 5pL⁴/(384D)、支点のせん断力と反力 pL/2。
    // 短辺の影響は中央まで指数的に減り、Ly/Lx = 8 で 1e-4 未満。許容は帯板の値の、たわみと曲げモーメントで
    // 5e-3、せん断力と反力で 2e-2（有限要素の離散化の誤差。ADR-0002）
    let lx = 2000.0;
    let p = simply_supported(lx, 8.0 * lx, 0.0, 10.0);
    let s = solve(&p).unwrap();
    let q = p.load.as_n_per_mm2();
    let center = at(&s, lx / 2.0, 4.0 * lx);
    let m_strip = q * lx * lx / 8.0;
    assert_within(
        center.mx.as_nmm_per_mm(),
        m_strip,
        5e-3 * m_strip,
        "中央の Mx",
    );
    assert_within(
        center.my.as_nmm_per_mm(),
        0.0,
        5e-3 * m_strip,
        "中央の My（ν = 0 なら 0）",
    );
    let w_strip = 5.0 * q * lx.powi(4) / (384.0 * rigidity(&p));
    assert_within(center.w.as_mm(), w_strip, 5e-3 * w_strip, "中央のたわみ");
    let v_strip = q * lx / 2.0;
    assert_within(
        at(&s, 0.0, 4.0 * lx).qx.as_n_per_mm(),
        v_strip,
        2e-2 * v_strip,
        "x = 0 の辺のせん断力",
    );
    // 辺の中央ではねじれが無いので、反力はせん断力と同じ pL/2
    let mid = s.edge(Edge::X0, 2)[1];
    assert_within(
        mid.reaction.as_n_per_mm(),
        v_strip,
        2e-2 * v_strip,
        "x = 0 の辺の反力",
    );
}

#[test]
fn simply_supported_edges_have_zero_deflection_and_zero_edge_moment() {
    // 単純支持の辺の節点は w と辺に沿う傾きを拘束するので、辺の上のたわみは形状関数の値として厳密に 0
    // （許容は中央のたわみの 1e-12）。辺に直交する曲げモーメントは 2 階の導関数を端の要素の中央から外挿した
    // 値で、厳密な 0 に離散化の誤差が残る（級数解との照合で辺の M の最大の 1e-5 の桁。許容は中央の 1e-4）
    let (lx, ly) = (3000.0, 2000.0);
    let s = solve(&simply_supported(lx, ly, 0.2, 10.0)).unwrap();
    let center = at(&s, lx / 2.0, ly / 2.0);
    let w_center = center.w.as_mm();
    let m_center = center.mx.as_nmm_per_mm();
    for (x, y) in [(0.0, 700.0), (lx, 1300.0), (1100.0, 0.0), (1900.0, ly)] {
        let r = at(&s, x, y);
        assert_within(
            r.w.as_mm(),
            0.0,
            1e-12 * w_center,
            &format!("({x}, {y}) のたわみ"),
        );
        let normal = if x == 0.0 || x == lx { r.mx } else { r.my };
        assert_within(
            normal.as_nmm_per_mm(),
            0.0,
            1e-4 * m_center,
            &format!("({x}, {y}) の辺に直交する曲げモーメント"),
        );
    }
    for e in Edge::ALL {
        for sample in s.edge(e, 20) {
            assert_within(
                sample.moment.as_nmm_per_mm(),
                0.0,
                1e-4 * m_center,
                &format!("{e} の曲げモーメント {}", sample.along.as_mm()),
            );
        }
    }
    // 単純支持の辺では傾きが残るので、辺からの距離 h のたわみは h に比例する: w(h)/w(2h) → 1/2。
    // h = 辺長の 1/200 で、h² 以上の項による比のずれは 1/200 程度なので許容 1e-2
    let h = lx / 200.0;
    let ratio = at(&s, h, 1000.0).w.as_mm() / at(&s, 2.0 * h, 1000.0).w.as_mm();
    assert_within(ratio, 0.5, 1e-2, "x = 0 の辺の近くの w(h)/w(2h)");
}

#[test]
fn edge_reactions_minus_corner_reactions_balance_the_total_load() {
    // 辺の反力の合計 − 隅反力の合計 = 荷重の合計（ADR-0004）。辺の反力の合計は節点の反力の和そのもので、
    // 隅の節点の反力は隅反力を足し戻してから辺に振り分けるので、丸め誤差（許容 1e-7）で一致する
    for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (4000.0, 2000.0)] {
        let p = simply_supported(lx, ly, 0.15, 8.0);
        let s = solve(&p).unwrap();
        let total = p.load.as_n_per_mm2() * lx * ly;
        assert_within(
            reactions_minus_corners(&s),
            total,
            1e-7 * total,
            &format!("{lx}×{ly} の釣り合い"),
        );
        // 隅反力があるので、辺の反力の合計は荷重の合計より大きい
        let reactions: f64 = Edge::ALL.iter().map(|&e| s.total_reaction(e).as_n()).sum();
        assert!(reactions > total, "{lx}×{ly} の辺の反力の合計");
        for c in Corner::ALL {
            assert!(s.corner_reaction(c).as_n() > 0.0, "{lx}×{ly} の {c:?}");
        }
        for e in Edge::ALL {
            let mid = s.edge(e, 2)[1];
            assert!(
                mid.reaction.as_n_per_mm() > 0.0,
                "{e} の反力が支える向きでない"
            );
        }
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
    let a = solve(&simply_supported(2000.0, 3000.0, 0.25, 10.0)).unwrap();
    let b = solve(&simply_supported(3000.0, 2000.0, 0.25, 10.0)).unwrap();
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
    for (ca, cb) in [
        (Corner::X0Y0, Corner::X0Y0),
        (Corner::XLxY0, Corner::X0YLy),
        (Corner::XLxYLy, Corner::XLxYLy),
        (Corner::X0YLy, Corner::XLxY0),
    ] {
        let ra = a.corner_reaction(ca).as_n();
        assert_within(
            ra,
            b.corner_reaction(cb).as_n(),
            1e-9 * ra,
            &format!("{ca:?} の隅反力"),
        );
    }
}

#[test]
fn a_square_plate_is_symmetric_under_the_symmetries_of_the_square() {
    // 四辺単純支持の正方形は、x と y の入れ替え、x の裏返し、y の裏返しで同じ板。要素の分割も対称なので、
    // 差は丸め誤差。せん断力は裏返す軸の向きで符号が変わる
    let l = 2500.0;
    let s = solve(&simply_supported(l, l, 0.3, 10.0)).unwrap();
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
    // 4 つの隅の隅反力は等しい
    let r = s.corner_reaction(Corner::X0Y0).as_n();
    for c in Corner::ALL {
        assert_within(s.corner_reaction(c).as_n(), r, 1e-9 * r, &format!("{c:?}"));
    }
}

#[test]
fn poisson_ratio_scales_the_deflection_and_mixes_the_moments() {
    // 自由辺の無い型では無次元の解が ν に依らない（ADR-0005）: w ∝ 1/D ∝ 1 − ν²、Mx(ν) = Mx(0) + ν·My(0)。
    // 有限要素でも同じ。ひずみエネルギーの ν の項 2ν (w_xx w_yy − w_xy²) は、要素の間で w と傾きが連続なので
    // 板全体では辺の上の値だけで決まり、単純支持の辺（w と辺に沿う傾きが 0）では 0 になる。差は連立の丸め
    // （許容 1e-9）。辺の反力と隅反力は換算に乗らないので見ない
    let (lx, ly) = (2000.0, 2800.0);
    let nu = 0.3;
    let s0 = solve(&simply_supported(lx, ly, 0.0, 10.0)).unwrap();
    let s1 = solve(&simply_supported(lx, ly, nu, 10.0)).unwrap();
    let scale = Scale::of(&s0);
    for (x, y) in [(1000.0, 1400.0), (300.0, 1400.0), (600.0, 2500.0)] {
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
        assert_within(
            p1.qx.as_n_per_mm(),
            p0.qx.as_n_per_mm(),
            1e-9 * scale.q,
            "Qx は ν に依らない",
        );
    }
}

#[test]
fn the_edge_maximum_of_a_square_plate_is_at_the_middle_of_the_edge() {
    let l = 2000.0;
    let s = solve(&simply_supported(l, l, 0.0, 10.0)).unwrap();
    for e in Edge::ALL {
        let m = s.edge_maximum(e, 100);
        let mid = s.edge(e, 100)[50];
        assert_eq!(m.reaction.along.as_mm(), l / 2.0, "{e} の反力の最大の位置");
        assert_eq!(m.reaction.value, mid.reaction, "{e} の反力の最大");
        assert_eq!(m.shear.along.as_mm(), l / 2.0, "{e} のせん断力の最大の位置");
        assert_eq!(m.shear.value, mid.shear, "{e} のせん断力の最大");
    }
    // 辺の上のせん断力は、内部の評価と同じ導関数から求めるので同じ値
    let edge = s.edge(Edge::X0, 2)[1];
    let point = at(&s, 0.0, l / 2.0);
    assert_eq!(edge.shear, point.qx, "辺の中央の Qx");
}
