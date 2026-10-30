//! 参照解（四辺固定と四辺単純支持の級数解。`solve_with_series`）と、有限要素（`solve`）との照合。
//! 参照解そのものは、理論から言える値（境界条件、帯板の極限）、独立な解（Navier の二重級数）、項数を
//! 変えたときの収束で確かめる。有限要素はその参照解と比べ、差が ADR-0002 の許容に収まることを見る

mod common;
use common::{
    POINT_QUANTITIES, assert_deviations_within_tolerance, assert_within, at, deviations,
    integrated_reaction, navier, plate, rigidity,
};
use rustruct_calc::kirchhoff_plate::{
    Plate, PlateError, PointResponse, Solution, SupportType, series, solve, solve_with_series,
};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};

const SSSS: Supports = Supports::all(Support::SimplySupported);
const FFFF: Supports = Supports::all(Support::Fixed);
/// 三辺固定一辺自由。自由辺は y = Ly
const FFFR: Supports = Supports {
    x0: Support::Fixed,
    y0: Support::Fixed,
    x_lx: Support::Fixed,
    y_ly: Support::Free,
};

/// 参照解を既定の項数で
fn reference(plate: &Plate) -> Solution {
    solve_with_series(plate, series::TERMS)
        .unwrap()
        .expect("参照解のある型")
}

#[test]
fn only_the_fixed_and_the_simply_supported_types_have_a_reference() {
    for ty in SupportType::ALL {
        let p = plate(2000.0, 3000.0, 0.2, 10.0, ty.representative());
        let result = solve_with_series(&p, series::TERMS);
        match ty {
            SupportType::FFFF | SupportType::SSSS => {
                assert_eq!(result.unwrap().unwrap().support_type(), ty);
            }
            SupportType::RRRR | SupportType::RRRS => {
                assert_eq!(result.err(), Some(PlateError::Unstable(ty)));
            }
            _ => assert!(result.unwrap().is_none(), "{}", ty.symbol()),
        }
    }
}

#[test]
fn the_simply_supported_series_matches_the_navier_double_series() {
    // Navier の二重級数は単一級数と別の展開で、同じ板の解に収束する。m・n を 799 まで（各 400 項）取ると、
    // Navier の打ち切りの差はたわみで相対 1e-12、曲げモーメントとせん断力で各量の最大の 1e-5 未満
    // （項を倍にすると曲げモーメントは 1/8、せん断力は 1/4 になる程度）。許容は各量の代表値の 1e-5
    let (lx, ly) = (2000.0, 3000.0);
    let p = plate(lx, ly, 0.2, 10.0, SSSS);
    let s = reference(&p);
    let w_scale = at(&s, lx / 2.0, ly / 2.0).w.as_mm();
    let m_scale = at(&s, lx / 2.0, ly / 2.0).mx.as_nmm_per_mm();
    let q_scale = at(&s, 0.0, ly / 2.0).qx.as_n_per_mm().abs();
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
        assert_within(ours.w.as_mm(), n.w, 1e-5 * w_scale, &format!("{what} の w"));
        assert_within(
            ours.mx.as_nmm_per_mm(),
            n.mx,
            1e-5 * m_scale,
            &format!("{what} の Mx"),
        );
        assert_within(
            ours.my.as_nmm_per_mm(),
            n.my,
            1e-5 * m_scale,
            &format!("{what} の My"),
        );
        assert_within(
            ours.qx.as_n_per_mm(),
            n.qx,
            1e-5 * q_scale,
            &format!("{what} の Qx"),
        );
        assert_within(
            ours.qy.as_n_per_mm(),
            n.qy,
            1e-5 * q_scale,
            &format!("{what} の Qy"),
        );
    }
    // 隅反力 R = 2 D (1 − ν) |w_xy|（隅）。隅の w_xy は Navier の各項が 1/(m² + n²)² で減るので速く収束する。
    // 単一級数の側は項が 1/β³ で減り、80 項の打ち切りが相対 1e-5 程度。許容は隅反力の 1e-4
    let expected = 2.0 * rigidity(&p) * (1.0 - 0.2) * navier(&p, 0.0, 0.0, 799).w_xy;
    for c in Corner::ALL {
        assert_within(
            s.corner_reaction(c).as_n(),
            expected,
            1e-4 * expected,
            &format!("{c:?} の隅反力"),
        );
    }
}

#[test]
fn the_fixed_series_satisfies_the_boundary_conditions_and_the_strip_limit() {
    // 辺の上のたわみは級数の各項が 0 なので厳密に 0（許容は中央のたわみの 1e-12）。固定辺では傾きも 0 なので、
    // 辺からの距離 h のたわみは h² に比例する: w(h)/w(2h) → 1/4（h = 辺長の 1/200、許容 1e-2）
    let lx = 3000.0;
    let s = reference(&plate(lx, 2000.0, 0.2, 10.0, FFFF));
    let w_center = at(&s, lx / 2.0, 1000.0).w.as_mm();
    for (x, y) in [(0.0, 700.0), (lx, 1300.0), (1100.0, 0.0), (1900.0, 2000.0)] {
        assert_within(
            at(&s, x, y).w.as_mm(),
            0.0,
            1e-12 * w_center,
            &format!("({x}, {y}) のたわみ"),
        );
    }
    let h = lx / 200.0;
    let ratio = at(&s, h, 1000.0).w.as_mm() / at(&s, 2.0 * h, 1000.0).w.as_mm();
    assert_within(ratio, 0.25, 1e-2, "x = 0 の辺の近くの w(h)/w(2h)");
    // Ly/Lx = 6 の帯板の極限。短辺方向の中央断面は両端が固定の梁: 中央 pL²/24、固定辺 −pL²/12、
    // たわみ pL⁴/(384D)、固定辺のせん断力 pL/2。長辺の影響は 1e-4 未満で、許容は帯板の値の 5e-4
    let lx = 2000.0;
    let p = plate(lx, 6.0 * lx, 0.0, 10.0, FFFF);
    let s = reference(&p);
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
    let w_strip = q * lx.powi(4) / (384.0 * rigidity(&p));
    assert_within(center.w.as_mm(), w_strip, 5e-4 * w_strip, "中央のたわみ");
    assert_within(
        edge.qx.as_n_per_mm(),
        q * lx / 2.0,
        5e-4 * q * lx / 2.0,
        "x = 0 の辺のせん断力",
    );
}

#[test]
fn the_series_edge_samples_integrate_to_the_total_reaction() {
    // 参照解の辺の反力の合計は辺の反力の台形則（1000 等分）で、`edge()` の細かい台形則と 1e-4 で一致する
    let s = reference(&plate(2000.0, 3000.0, 0.2, 10.0, FFFF));
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

/// 項数 N と 2N の差が、隅の近くを除いて各量の絶対値の最大の 1e-3 以下。`excluded` は除くグリッドの点
fn assert_doubling_the_terms_changes_nothing(
    p: &Plate,
    what: &str,
    excluded: impl Fn(&PointResponse) -> bool,
) {
    let a = solve_with_series(p, series::TERMS).unwrap().unwrap();
    let b = solve_with_series(p, 2 * series::TERMS).unwrap().unwrap();
    let (ga, gb) = (a.grid(20, 20), b.grid(20, 20));
    let mut m_scale = 0.0;
    for (name, f) in POINT_QUANTITIES {
        let scale = ga.iter().map(|p| f(p).abs()).fold(0.0, f64::max);
        if name.starts_with('M') {
            m_scale = f64::max(m_scale, scale);
        }
        for (pa, pb) in ga.iter().zip(&gb).filter(|(p, _)| !excluded(p)) {
            assert_within(
                f(pa),
                f(pb),
                1e-3 * scale,
                &format!("{what} の {name} ({}, {})", pa.x.as_mm(), pa.y.as_mm()),
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
        for (sa, sb) in ea.iter().zip(&eb).skip(5).take(91) {
            assert_within(
                sa.reaction.as_n_per_mm(),
                sb.reaction.as_n_per_mm(),
                1e-3 * v_scale,
                &format!("{what} の {e} の反力 {}", sa.along.as_mm()),
            );
            assert_within(
                sa.shear.as_n_per_mm(),
                sb.shear.as_n_per_mm(),
                1e-3 * q_scale,
                &format!("{what} の {e} のせん断力 {}", sa.along.as_mm()),
            );
            // 辺の曲げモーメントは板全体の最大に対して見る。単純支持の辺では厳密に 0
            assert_within(
                sa.moment.as_nmm_per_mm(),
                sb.moment.as_nmm_per_mm(),
                1e-3 * m_scale,
                &format!("{what} の {e} の曲げモーメント {}", sa.along.as_mm()),
            );
        }
    }
    for c in Corner::ALL {
        let r = a.corner_reaction(c).as_n();
        assert_within(
            r,
            b.corner_reaction(c).as_n(),
            1e-3 * r.abs(),
            &format!("{what} の {c:?} の隅反力"),
        );
    }
}

#[test]
fn doubling_the_terms_changes_nothing_beyond_the_tolerance() {
    // 参照解の項数の決め方（ADR-0006 の 2026-10-23 より前の節）。縦横比 1・2・4 で、21×21 のグリッドと辺の
    // 101 点（両端から 5 % を除く）で、N = TERMS と 2N の差がその量の絶対値の最大の 1e-3 以下。
    // 四辺固定は隅から辺長の 5 % 以内を除く（級数が隅の近くで振れる）。四辺単純支持は隅の点そのものだけを除く
    // （せん断力の厳密な値 0 に、級数が 1/N でしか近づかない）
    let lx = 2000.0;
    for ratio in [1.0, 2.0, 4.0] {
        let ly = ratio * lx;
        assert_doubling_the_terms_changes_nothing(
            &plate(lx, ly, 0.2, 10.0, FFFF),
            &format!("FFFF 比 {ratio}"),
            |p| {
                let (x, y) = (p.x.as_mm(), p.y.as_mm());
                common::near_corner(x, y, lx, ly)
            },
        );
        assert_doubling_the_terms_changes_nothing(
            &plate(lx, ly, 0.2, 10.0, SSSS),
            &format!("SSSS 比 {ratio}"),
            |p| [0.0, lx].contains(&p.x.as_mm()) && [0.0, ly].contains(&p.y.as_mm()),
        );
    }
}

#[test]
fn the_series_edge_reactions_converge_as_one_over_the_square_of_the_terms() {
    // 辺の反力は辺に沿う向きの級数で求めるので（ADR-0002 の級数の節）、打ち切りの誤差が項数の 2 乗に反比例
    // して減る。項数 2560 の値を基準に、項数を倍にするごとの誤差の比が 4 に近い（3.5〜4.5）ことを辺の中央と
    // 1/4 点で見る
    let p = plate(2000.0, 3000.0, 0.2, 10.0, SSSS);
    for e in [Edge::X0, Edge::Y0] {
        let reference = solve_with_series(&p, 2560).unwrap().unwrap().edge(e, 4);
        for i in [1, 2] {
            let error = |terms: usize| {
                let v = solve_with_series(&p, terms).unwrap().unwrap().edge(e, 4)[i]
                    .reaction
                    .as_n_per_mm();
                (v - reference[i].reaction.as_n_per_mm()).abs()
            };
            for terms in [20, 40, 80] {
                let ratio = error(terms) / error(2 * terms);
                assert!(
                    (3.5..=4.5).contains(&ratio),
                    "{e} の {i}/4 点で、項数 {terms} から倍にしたときの誤差の比 {ratio}"
                );
            }
        }
    }
}

#[test]
fn the_finite_elements_match_the_series_solutions() {
    // 有限要素（`solve`）と参照解の照合。縦横比 1・2・4、ν = 0 と 0.2 で、隅から辺長の 5 % を除いた範囲の
    // w・Mx・My が 0.5 % 以内、Qx・Qy・辺の反力・隅反力が 2 % 以内（その量の絶対値の最大に対して。ADR-0002）
    let lx = 2000.0;
    for (symbol, supports) in [("SSSS", SSSS), ("FFFF", FFFF)] {
        for nu in [0.0, 0.2] {
            for ratio in [1.0, 2.0, 4.0] {
                let p = plate(lx, ratio * lx, nu, 10.0, supports);
                assert_deviations_within_tolerance(
                    &reference(&p),
                    &solve(&p).unwrap(),
                    &format!("{symbol} ν = {nu} 比 {ratio}"),
                );
            }
        }
    }
}

#[test]
#[ignore = "参照解との差、分割数を倍にしたときの差、計算時間を出す。cargo test --release -- --ignored --nocapture で見る"]
fn report_deviations_and_timing() {
    use rustruct_calc::kirchhoff_plate::{DIVISIONS, solve_with_divisions};
    let lx = 2000.0;
    for (symbol, supports) in [("SSSS", SSSS), ("FFFF", FFFF), ("FFFR", FFFR)] {
        for nu in [0.0, 0.2] {
            for ratio in [1.0, 2.0, 4.0] {
                let p = plate(lx, ratio * lx, nu, 10.0, supports);
                let t = std::time::Instant::now();
                let f = solve(&p).unwrap();
                let solved = t.elapsed();
                let t = std::time::Instant::now();
                let _ = f.grid(50, 50);
                for e in Edge::ALL {
                    let _ = f.edge(e, 100);
                }
                let evaluated = t.elapsed();
                println!(
                    "== {symbol} ν = {nu} 比 {ratio}: 解 {solved:?}、51×51 と辺 4 本の評価 {evaluated:?}"
                );
                let t = std::time::Instant::now();
                let series = solve_with_series(&p, series::TERMS).unwrap();
                if let Some(series) = series {
                    let solved = t.elapsed();
                    let t = std::time::Instant::now();
                    let _ = series.grid(50, 50);
                    for e in Edge::ALL {
                        let _ = series.edge(e, 100);
                    }
                    println!(
                        "   参照解: 解 {solved:?}、51×51 と辺 4 本の評価 {:?}",
                        t.elapsed()
                    );
                    for d in deviations(&series, &f) {
                        println!("  {:<14} {:.2e} at {}", d.name, d.ratio, d.at);
                    }
                }
                if nu == 0.2 {
                    let fine = solve_with_divisions(&p, 2 * DIVISIONS).unwrap();
                    println!("   分割数 {DIVISIONS} と {} の差", 2 * DIVISIONS);
                    for d in deviations(&fine, &f) {
                        println!("  {:<14} {:.2e} at {}", d.name, d.ratio, d.at);
                    }
                }
            }
        }
    }
}
