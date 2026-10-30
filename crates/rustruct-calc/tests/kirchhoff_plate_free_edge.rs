//! 自由辺のある型の Kirchhoff 板。級数解が無いので、期待値は理論から言える値（帯板の極限、辺に沿う値の滑らかさ）
//! だけ。入口 `solve`（有限要素）からだけ確かめる。19 型に共通の照合（梁の解、釣り合い、対称、隅反力が 0 に
//! なる隅を含む）は `kirchhoff_plate_all_types.rs`

mod common;
use common::{EDGE_QUANTITIES, assert_within, at, plate, rigidity};
use rustruct_calc::kirchhoff_plate::{EdgeSample, solve};
use rustruct_types::kirchhoff_plate::{Edge, Support, Supports};

/// 三辺固定一辺自由。自由辺は y = Ly
const FFFR: Supports = Supports {
    x0: Support::Fixed,
    y0: Support::Fixed,
    x_lx: Support::Fixed,
    y_ly: Support::Free,
};

#[test]
fn a_long_three_fixed_one_free_plate_behaves_as_a_fixed_strip_across_the_short_span() {
    // 三辺固定一辺自由で Ly/Lx = 6、ν = 0。y = 0 の固定辺から離れた断面は両端固定の梁で、自由辺（y = Ly）の
    // 上でもそのまま（ν = 0 なら帯板の解が自由辺の条件を満たす）。中央 pL²/24、固定辺 −pL²/12、
    // たわみ pL⁴/(384D)、固定辺のせん断力と反力 pL/2。許容はたわみと曲げモーメントが 0.5 %、
    // せん断力と反力が 2 %（有限要素の離散化の誤差。ADR-0002）
    let lx = 2000.0;
    let p = plate(lx, 6.0 * lx, 0.0, 10.0, FFFR);
    let s = solve(&p).unwrap();
    let q = p.load.as_n_per_mm2();
    let m_strip = q * lx * lx / 24.0;
    let w_strip = q * lx.powi(4) / (384.0 * rigidity(&p));
    for y in [3.0 * lx, 5.0 * lx, 6.0 * lx] {
        let centre = at(&s, lx / 2.0, y);
        let edge = at(&s, 0.0, y);
        assert_within(
            centre.mx.as_nmm_per_mm(),
            m_strip,
            5e-3 * m_strip,
            &format!("y = {y} の中央の Mx"),
        );
        assert_within(
            edge.mx.as_nmm_per_mm(),
            -2.0 * m_strip,
            5e-3 * 2.0 * m_strip,
            &format!("y = {y} の固定辺の Mx"),
        );
        assert_within(
            centre.my.as_nmm_per_mm(),
            0.0,
            5e-3 * m_strip,
            &format!("y = {y} の中央の My"),
        );
        assert_within(
            centre.w.as_mm(),
            w_strip,
            5e-3 * w_strip,
            &format!("y = {y} の中央のたわみ"),
        );
        assert_within(
            edge.qx.as_n_per_mm(),
            q * lx / 2.0,
            2e-2 * q * lx / 2.0,
            &format!("y = {y} の固定辺のせん断力"),
        );
    }
    let x0 = s.edge(Edge::X0, 12);
    assert_within(
        x0[6].reaction.as_n_per_mm(),
        q * lx / 2.0,
        2e-2 * q * lx / 2.0,
        "x = 0 の辺の中央の反力",
    );
    // 自由辺の反力は 0、曲げモーメントは固定辺のモーメントに比べて小さい
    for sample in s.edge(Edge::YLy, 20) {
        assert_eq!(sample.reaction.as_n_per_mm(), 0.0);
    }
    for sample in s.edge(Edge::YLy, 20).iter().skip(1).take(19) {
        assert_within(
            sample.moment.as_nmm_per_mm(),
            0.0,
            5e-3 * 2.0 * m_strip,
            "自由辺の My",
        );
    }
}

#[test]
fn the_shear_and_reaction_along_a_fixed_edge_are_smooth_when_nu_is_not_zero() {
    // 自由辺のある型を ν ≠ 0 で解いても、固定辺のせん断力と反力は辺に沿って振れない（ADR-0005）。両端から
    // 5 % を除いた範囲で、隣り合う評価点の差の符号が変わるのは 1 回以下（辺の中央で向きが変わるだけ）
    let p = plate(2000.0, 2000.0, 0.2, 10.0, FFFR);
    let s = solve(&p).unwrap();
    for e in [Edge::X0, Edge::Y0, Edge::XLx] {
        let samples: Vec<EdgeSample> = s.edge(e, 100).into_iter().skip(5).take(91).collect();
        for (name, f) in [EDGE_QUANTITIES[1], EDGE_QUANTITIES[2]] {
            let differences: Vec<f64> = samples.windows(2).map(|w| f(&w[1]) - f(&w[0])).collect();
            let sign_changes = differences.windows(2).filter(|d| d[0] * d[1] < 0.0).count();
            assert!(
                sign_changes <= 1,
                "{e} の {name} の向きが {sign_changes} 回変わる"
            );
        }
    }
}
