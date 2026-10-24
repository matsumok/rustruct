//! 四辺単純支持（SSSS）の Kirchhoff 板。入口 `solve`（有限要素）からだけ確かめる。期待値は理論から言える値
//! （境界条件、隅反力の向き）と、独立な解（Navier の二重正弦級数）だけ。19 型に共通の照合は
//! `kirchhoff_plate_all_types.rs`。
//! 級数解との照合は `kirchhoff_plate_series_reference.rs`

mod common;
use common::{assert_within, at, navier, plate, rigidity};
use rustruct_calc::kirchhoff_plate::{Plate, SupportType, solve};
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
    // 辺に沿った応答の、辺に直交する曲げモーメントは境界条件そのものなので、離散化の屑を残さずちょうど 0
    for e in Edge::ALL {
        for sample in s.edge(e, 20) {
            assert_eq!(
                sample.moment.as_nmm_per_mm(),
                0.0,
                "{e} の曲げモーメント {}",
                sample.along.as_mm()
            );
        }
        assert_eq!(
            s.edge_maximum(e, 20).moment.value.as_nmm_per_mm(),
            0.0,
            "{e} の曲げモーメントの最大"
        );
    }
    // 単純支持の辺では傾きが残るので、辺からの距離 h のたわみは h に比例する: w(h)/w(2h) → 1/2。
    // h = 辺長の 1/200 で、h² 以上の項による比のずれは 1/200 程度なので許容 1e-2
    let h = lx / 200.0;
    let ratio = at(&s, h, 1000.0).w.as_mm() / at(&s, 2.0 * h, 1000.0).w.as_mm();
    assert_within(ratio, 0.5, 1e-2, "x = 0 の辺の近くの w(h)/w(2h)");
}

#[test]
fn the_corners_hold_the_plate_down_and_the_edges_hold_it_up() {
    // 単純支持の隅では板が浮き上がろうとし、支持が押さえる（隅反力が正）。釣り合いは
    // `kirchhoff_plate_all_types.rs`
    for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (4000.0, 2000.0)] {
        let p = simply_supported(lx, ly, 0.15, 8.0);
        let s = solve(&p).unwrap();
        let total = p.load.as_n_per_mm2() * lx * ly;
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
