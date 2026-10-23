//! 自由辺のある型の Kirchhoff 板。級数解が無いので、期待値は理論から言える値（帯板の極限、梁の解、釣り合い、
//! 対称、隅反力が 0 になる隅）だけ。入口 `solve`（有限要素）からだけ確かめる

mod common;
use common::{EDGE_QUANTITIES, assert_within, at, plate, reactions_minus_corners, rigidity};
use rustruct_calc::kirchhoff_plate::{EdgeSample, PointResponse, Solution, solve};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};

/// 三辺固定一辺自由。自由辺は y = Ly
const FFFR: Supports = Supports {
    x0: Support::Fixed,
    y0: Support::Fixed,
    x_lx: Support::Fixed,
    y_ly: Support::Free,
};
/// 片持ち。固定辺は x = 0
const FRRR: Supports = Supports {
    x0: Support::Fixed,
    y0: Support::Free,
    x_lx: Support::Free,
    y_ly: Support::Free,
};
/// 向かい合う 2 辺が単純支持、残る 2 辺が自由
const RSRS: Supports = Supports {
    x0: Support::Free,
    y0: Support::SimplySupported,
    x_lx: Support::Free,
    y_ly: Support::SimplySupported,
};
const FSRS: Supports = Supports {
    x0: Support::Fixed,
    y0: Support::SimplySupported,
    x_lx: Support::Free,
    y_ly: Support::SimplySupported,
};

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
fn a_cantilever_plate_with_zero_poisson_ratio_is_a_cantilever_beam() {
    // 片持ちの板（FRRR）は ν = 0 なら梁の解と一致する（ADR-0002）: 固定辺から x の断面で
    // Mx = −p (L − x)²/2、Qx = p (L − x)、先端のたわみ pL⁴/(8D)。y に依らない。許容はたわみと曲げモーメントが
    // 0.5 %、せん断力が 2 %（固定辺の値に対して）
    let (lx, ly) = (2000.0, 1500.0);
    let p = plate(lx, ly, 0.0, 10.0, FRRR);
    let s = solve(&p).unwrap();
    let q = p.load.as_n_per_mm2();
    let m_root = q * lx * lx / 2.0;
    let w_tip = q * lx.powi(4) / (8.0 * rigidity(&p));
    for y in [0.0, 400.0, ly / 2.0, ly] {
        for x in [0.0, 500.0, 1000.0, 1500.0] {
            let r = at(&s, x, y);
            let what = format!("({x}, {y})");
            assert_within(
                r.mx.as_nmm_per_mm(),
                -q * (lx - x) * (lx - x) / 2.0,
                5e-3 * m_root,
                &format!("{what} の Mx"),
            );
            assert_within(
                r.my.as_nmm_per_mm(),
                0.0,
                5e-3 * m_root,
                &format!("{what} の My"),
            );
            assert_within(
                r.qx.as_n_per_mm(),
                q * (lx - x),
                2e-2 * q * lx,
                &format!("{what} の Qx"),
            );
            // 梁のたわみ w = p x² (6L² − 4Lx + x²) / (24D)
            let w_beam = q * x * x * (6.0 * lx * lx - 4.0 * lx * x + x * x) / (24.0 * rigidity(&p));
            assert_within(r.w.as_mm(), w_beam, 5e-3 * w_tip, &format!("{what} の w"));
        }
        assert_within(
            at(&s, lx, y).w.as_mm(),
            w_tip,
            5e-3 * w_tip,
            &format!("y = {y} の先端のたわみ"),
        );
    }
    // 固定辺の反力は pL で、荷重の合計を一辺で受ける
    for sample in s.edge(Edge::X0, 10).iter().skip(1).take(9) {
        assert_within(
            sample.reaction.as_n_per_mm(),
            q * lx,
            2e-2 * q * lx,
            &format!("x = 0 の辺の反力 {}", sample.along.as_mm()),
        );
    }
    let total = q * lx * ly;
    assert_within(
        s.total_reaction(Edge::X0).as_n(),
        total,
        1e-7 * total,
        "固定辺の反力の合計",
    );
}

#[test]
fn edge_reactions_minus_corner_reactions_balance_the_total_load_to_round_off() {
    // 辺の反力の合計 − 隅反力の合計 = 荷重の合計（ADR-0004）が、辺の反力の合計は節点の反力の和そのものなので
    // 丸め誤差（連立の大きさで 1e-10 の桁。許容 1e-7）で成り立つ。固定辺を含む隅と自由辺どうしの隅の隅反力は 0
    for supports in [FFFR, FRRR, RSRS, FSRS] {
        for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (4000.0, 2000.0)] {
            let p = plate(lx, ly, 0.15, 8.0, supports);
            let s = solve(&p).unwrap();
            let total = p.load.as_n_per_mm2() * lx * ly;
            assert_within(
                reactions_minus_corners(&s),
                total,
                1e-7 * total,
                &format!("{} {lx}×{ly} の釣り合い", supports.symbol()),
            );
            for c in Corner::ALL {
                let (ex, ey) = c.edges();
                let (sx, sy) = (supports.at(ex), supports.at(ey));
                let both_free = sx == Support::Free && sy == Support::Free;
                if sx == Support::Fixed || sy == Support::Fixed || both_free {
                    assert_eq!(
                        s.corner_reaction(c).as_n(),
                        0.0,
                        "{} の {c:?}",
                        supports.symbol()
                    );
                }
            }
            // 自由辺の反力は 0
            for e in Edge::ALL {
                if supports.at(e) == Support::Free {
                    assert_eq!(
                        s.total_reaction(e).as_n(),
                        0.0,
                        "{} の {e}",
                        supports.symbol()
                    );
                }
            }
        }
    }
}

#[test]
fn swapping_lx_and_ly_swaps_x_and_y_in_the_results() {
    // 2000×3000 の (x, y) と 3000×2000 の (y, x) は、支持条件も入れ替えれば同じ板。要素の分割も入れ替わる
    // だけなので、差は丸め誤差
    let (lx, ly) = (2000.0, 3000.0);
    let transposed = |s: Supports| Supports {
        x0: s.y0,
        y0: s.x0,
        x_lx: s.y_ly,
        y_ly: s.x_lx,
    };
    for supports in [FFFR, FRRR, FSRS] {
        let a = solve(&plate(lx, ly, 0.2, 10.0, supports)).unwrap();
        let b = solve(&plate(ly, lx, 0.2, 10.0, transposed(supports))).unwrap();
        let scale = Scale::of(&a);
        for (x, y) in [
            (500.0, 700.0),
            (1000.0, 1500.0),
            (1900.0, 2900.0),
            (0.0, 1200.0),
        ] {
            let (ra, rb) = (at(&a, x, y), at(&b, y, x));
            let what = format!("{} ({x}, {y})", supports.symbol());
            assert_within(
                ra.w.as_mm(),
                rb.w.as_mm(),
                1e-9 * scale.w,
                &format!("{what} の w"),
            );
            assert_within(
                ra.mx.as_nmm_per_mm(),
                rb.my.as_nmm_per_mm(),
                1e-9 * scale.m,
                &format!("{what} の Mx"),
            );
            assert_within(
                ra.my.as_nmm_per_mm(),
                rb.mx.as_nmm_per_mm(),
                1e-9 * scale.m,
                &format!("{what} の My"),
            );
            assert_within(
                ra.qx.as_n_per_mm(),
                rb.qy.as_n_per_mm(),
                1e-9 * scale.q,
                &format!("{what} の Qx"),
            );
            assert_within(
                ra.qy.as_n_per_mm(),
                rb.qx.as_n_per_mm(),
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
                    &format!("{} {ea} の反力", supports.symbol()),
                );
            }
        }
    }
}

#[test]
fn a_three_fixed_one_free_square_is_symmetric_about_the_middle_of_the_free_edge() {
    // 三辺固定一辺自由の正方形は x の裏返しで同じ板。要素の分割も対称なので差は丸め誤差
    let l = 2000.0;
    let s = solve(&plate(l, l, 0.2, 10.0, FFFR)).unwrap();
    let scale = Scale::of(&s);
    for (x, y) in [(300.0, 700.0), (100.0, 1900.0), (0.0, 1200.0), (600.0, l)] {
        let (r, fx) = (at(&s, x, y), at(&s, l - x, y));
        assert_within(r.w.as_mm(), fx.w.as_mm(), 1e-9 * scale.w, "x の裏返しの w");
        assert_within(
            r.mx.as_nmm_per_mm(),
            fx.mx.as_nmm_per_mm(),
            1e-9 * scale.m,
            "x の裏返しの Mx",
        );
        assert_within(
            r.qx.as_n_per_mm(),
            -fx.qx.as_n_per_mm(),
            1e-9 * scale.q,
            "x の裏返しの Qx",
        );
        assert_within(
            r.qy.as_n_per_mm(),
            fx.qy.as_n_per_mm(),
            1e-9 * scale.q,
            "x の裏返しの Qy",
        );
    }
    let (x0, x_lx) = (s.edge(Edge::X0, 10), s.edge(Edge::XLx, 10));
    for (a, b) in x0.iter().zip(&x_lx) {
        assert_within(
            a.reaction.as_n_per_mm(),
            b.reaction.as_n_per_mm(),
            1e-9 * scale.q,
            "x = 0 と x = Lx の反力",
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
