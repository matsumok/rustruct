//! 四辺単純支持（SSSS）の Kirchhoff 板。入口 `solve` からだけ確かめる。期待値は理論から言える値（帯板の極限、
//! 境界条件、釣り合い、対称、ν の換算）と、独立な解（Navier の二重正弦級数）、項数を変えたときの収束だけで、
//! 文献の数値は置かない。

mod common;
use std::f64::consts::PI;

use common::assert_within;
use rustruct_calc::kirchhoff_plate::{
    Plate, PointResponse, Solution, SupportType, TERMS, solve, solve_with_terms,
};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};
use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};

/// 辺長は mm、p は kN/m²。t = 100 mm、E = 20 000 N/mm²
fn simply_supported(lx: f64, ly: f64, nu: f64, p: f64) -> Plate {
    Plate {
        lx: Length::mm(lx),
        ly: Length::mm(ly),
        thickness: Length::mm(100.0),
        young_modulus: Stress::n_per_mm2(20_000.0),
        poisson_ratio: PoissonRatio::new(nu).unwrap(),
        load: AreaLoad::kn_per_m2(p),
        supports: Supports::all(Support::SimplySupported),
    }
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
fn four_simply_supported_edges_are_the_ssss_type() {
    let s = solve(&simply_supported(2000.0, 3000.0, 0.0, 10.0)).unwrap();
    assert_eq!(s.support_type(), SupportType::SSSS);
}

/// Navier の二重正弦級数で求めた、角原点の点 (x, y) の応答と角の w_xy。m・n は奇数を `max` まで。
/// w = Σ A sin αx sin βy、α = mπ/Lx、β = nπ/Ly、A = 16 p / (π² m n D (α² + β²)²)。
/// 曲げモーメントとせん断力は解き方と同じ定義（M = −D (w_xx + ν w_yy)、Q = −D (w_xxx + w_xyy)）から項ごとに微分する
struct Navier {
    w: f64,
    mx: f64,
    my: f64,
    qx: f64,
    qy: f64,
    w_xy: f64,
}

fn navier(plate: &Plate, x: f64, y: f64, max: usize) -> Navier {
    let (lx, ly) = (plate.lx.as_mm(), plate.ly.as_mm());
    let (p, d, nu) = (
        plate.load.as_n_per_mm2(),
        rigidity(plate),
        plate.poisson_ratio.value(),
    );
    let mut r = Navier {
        w: 0.0,
        mx: 0.0,
        my: 0.0,
        qx: 0.0,
        qy: 0.0,
        w_xy: 0.0,
    };
    for m in (1..=max).step_by(2) {
        for n in (1..=max).step_by(2) {
            let alpha = m as f64 * PI / lx;
            let beta = n as f64 * PI / ly;
            let k = alpha * alpha + beta * beta;
            let amp = 16.0 * p / (PI * PI * (m * n) as f64 * d * k * k);
            let (sx, cx) = (alpha * x).sin_cos();
            let (sy, cy) = (beta * y).sin_cos();
            r.w += amp * sx * sy;
            r.mx += d * amp * (alpha * alpha + nu * beta * beta) * sx * sy;
            r.my += d * amp * (beta * beta + nu * alpha * alpha) * sx * sy;
            r.qx += d * amp * alpha * k * cx * sy;
            r.qy += d * amp * beta * k * sx * cy;
            r.w_xy += amp * alpha * beta * cx * cy;
        }
    }
    r
}

#[test]
fn the_interior_matches_the_navier_double_series() {
    // Navier の二重級数は単一級数と別の展開で、同じ板の解に収束する。m・n を 799 まで（各 400 項）取ると、
    // Navier の打ち切りの差はたわみで相対 1e-12、曲げモーメントとせん断力で各量の最大の 1e-5 未満
    // （項を倍にすると曲げモーメントは 1/8、せん断力は 1/4 になる程度）。許容は各量の代表値の 1e-5
    let (lx, ly) = (2000.0, 3000.0);
    let p = simply_supported(lx, ly, 0.2, 10.0);
    let s = solve(&p).unwrap();
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
}

#[test]
fn corner_reactions_match_the_navier_twist_at_the_corner() {
    // 隅反力 R = 2 D (1 − ν) |w_xy|（角）。角の w_xy は Navier の各項が 1/(m² + n²)² で減るので速く収束する。
    // 単一級数の側は項が 1/β³ で減り、80 項の打ち切りが相対 1e-5 程度。許容は隅反力の 1e-4
    let (lx, ly) = (2000.0, 3000.0);
    let p = simply_supported(lx, ly, 0.2, 10.0);
    let s = solve(&p).unwrap();
    let n = navier(&p, 0.0, 0.0, 799);
    let expected = 2.0 * rigidity(&p) * (1.0 - 0.2) * n.w_xy;
    for c in Corner::ALL {
        assert_within(
            s.corner_reaction(c).as_n(),
            expected,
            1e-4 * expected,
            &format!("{c:?} の隅反力"),
        );
    }
    // 単純支持の角では板が浮き上がろうとし、支持が押さえる（正）
    assert!(expected > 0.0);
}

#[test]
fn a_long_plate_behaves_as_a_simply_supported_strip_across_the_short_span() {
    // Ly/Lx = 8。短辺方向の中央断面は単純支持の梁: 中央 pL²/8、たわみ 5pL⁴/(384D)、支点のせん断力と反力 pL/2。
    // 短辺の影響は中央まで指数的に減り、Ly/Lx = 8 で 1e-4 未満（6 では 1e-3 の桁）。許容は帯板の値の 2e-4
    let lx = 2000.0;
    let p = simply_supported(lx, 8.0 * lx, 0.0, 10.0);
    let s = solve(&p).unwrap();
    let q = p.load.as_n_per_mm2();
    let center = at(&s, lx / 2.0, 4.0 * lx);
    let m_strip = q * lx * lx / 8.0;
    assert_within(
        center.mx.as_nmm_per_mm(),
        m_strip,
        2e-4 * m_strip,
        "中央の Mx",
    );
    assert_within(
        center.my.as_nmm_per_mm(),
        0.0,
        2e-4 * m_strip,
        "中央の My（ν = 0 なら 0）",
    );
    let w_strip = 5.0 * q * lx.powi(4) / (384.0 * rigidity(&p));
    assert_within(center.w.as_mm(), w_strip, 2e-4 * w_strip, "中央のたわみ");
    let v_strip = q * lx / 2.0;
    assert_within(
        at(&s, 0.0, 4.0 * lx).qx.as_n_per_mm(),
        v_strip,
        2e-4 * v_strip,
        "x = 0 の辺のせん断力",
    );
    // 辺の中央ではねじれが無いので、反力はせん断力と同じ pL/2
    let mid = s.edge(Edge::X0, 2)[1];
    assert_within(
        mid.reaction.as_n_per_mm(),
        v_strip,
        2e-4 * v_strip,
        "x = 0 の辺の反力",
    );
}

#[test]
fn simply_supported_edges_have_zero_deflection_and_zero_edge_moment() {
    // 内部の評価は、辺の上ではその辺を級数の三角関数の向きの端に持つ級数を選び、各項が 0 になるので丸めだけ。
    // 許容は中央のたわみ・曲げモーメントの 1e-12
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
            1e-12 * m_center,
            &format!("({x}, {y}) の辺に直交する曲げモーメント"),
        );
    }
    // 辺に沿った応答の、辺に直交する曲げモーメントは境界条件そのものなので、級数の屑を残さずちょうど 0
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
fn edge_reactions_minus_corner_reactions_balance_the_total_load() {
    // 辺の反力の合計 − 隅反力の合計 = 荷重の合計（ADR-0004）。辺の反力は辺の上で滑らかで角で 0 になるので、
    // 400 等分の台形則の誤差は荷重の合計の 1e-5 未満。許容は 1e-4
    for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (4000.0, 2000.0)] {
        let p = simply_supported(lx, ly, 0.15, 8.0);
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
            1e-4 * total,
            &format!("{lx}×{ly} の釣り合い"),
        );
        // 隅反力があるので、辺の反力の合計は荷重の合計より大きい
        assert!(reactions > total, "{lx}×{ly} の辺の反力の合計");
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
fn edge_reactions_converge_as_one_over_the_square_of_the_terms() {
    // 辺の反力は辺に沿う向きの級数で求めるので（ADR-0002）、打ち切りの誤差が項数の 2 乗に反比例して減る。
    // 項数 2560 の値を基準に、項数を倍にするごとの誤差の比が 4 に近い（3.5〜4.5）ことを辺の中央と 1/4 点で見る
    let p = simply_supported(2000.0, 3000.0, 0.2, 10.0);
    for e in [Edge::X0, Edge::Y0] {
        let reference = solve_with_terms(&p, 2560).unwrap().edge(e, 4);
        for i in [1, 2] {
            let error = |terms: usize| {
                let v = solve_with_terms(&p, terms).unwrap().edge(e, 4)[i]
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
fn swapping_lx_and_ly_swaps_x_and_y_in_the_results() {
    // 2000×3000 の (x, y) と 3000×2000 の (y, x) は同じ板。Lx で無次元化した別の級数で解くので、差は打ち切り
    // 誤差。Navier との照合と同じく、各量の代表値の 1e-5 を許す
    let a = solve(&simply_supported(2000.0, 3000.0, 0.25, 10.0)).unwrap();
    let b = solve(&simply_supported(3000.0, 2000.0, 0.25, 10.0)).unwrap();
    let w_scale = at(&a, 1000.0, 1500.0).w.as_mm();
    let m_scale = at(&a, 1000.0, 1500.0).mx.as_nmm_per_mm();
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
            1e-5 * w_scale,
            &format!("{what} の w"),
        );
        assert_within(
            pa.mx.as_nmm_per_mm(),
            pb.my.as_nmm_per_mm(),
            1e-5 * m_scale,
            &format!("{what} の Mx"),
        );
        assert_within(
            pa.my.as_nmm_per_mm(),
            pb.mx.as_nmm_per_mm(),
            1e-5 * m_scale,
            &format!("{what} の My"),
        );
        assert_within(
            pa.qx.as_n_per_mm(),
            pb.qy.as_n_per_mm(),
            1e-5 * q_scale,
            &format!("{what} の Qx"),
        );
        assert_within(
            pa.qy.as_n_per_mm(),
            pb.qx.as_n_per_mm(),
            1e-5 * q_scale,
            &format!("{what} の Qy"),
        );
    }
    let va = a.edge(Edge::X0, 10);
    let vb = b.edge(Edge::Y0, 10);
    for (p, q) in va.iter().zip(&vb) {
        assert_within(
            p.reaction.as_n_per_mm(),
            q.reaction.as_n_per_mm(),
            1e-5 * q_scale,
            "x = 0 と y = 0 の反力",
        );
    }
    for (ca, cb) in [
        (Corner::X0Y0, Corner::X0Y0),
        (Corner::XLxY0, Corner::X0YLy),
        (Corner::XLxYLy, Corner::XLxYLy),
        (Corner::X0YLy, Corner::XLxY0),
    ] {
        // 隅反力の打ち切りは Navier との照合と同じく相対 1e-5 程度で、二つの解でずれ方が違う。許容は 1e-4
        let ra = a.corner_reaction(ca).as_n();
        assert_within(
            ra,
            b.corner_reaction(cb).as_n(),
            1e-4 * ra,
            &format!("{ca:?} の隅反力"),
        );
    }
}

#[test]
fn a_square_plate_is_symmetric_in_x_and_y_and_about_its_center() {
    // 同じ解の中の対称なので、差は丸めだけ。許容は各量の代表値の 1e-9。
    // x と y の入れ替えは別の向きの級数を通ることがあるので、打ち切りの差（1e-5）を許す
    let l = 2500.0;
    let s = solve(&simply_supported(l, l, 0.3, 10.0)).unwrap();
    let w_scale = at(&s, l / 2.0, l / 2.0).w.as_mm();
    let m_scale = at(&s, l / 2.0, l / 2.0).mx.as_nmm_per_mm();
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
            1e-5 * m_scale,
            "Mx(x, y) = My(y, x)",
        );
        assert_within(
            p.qx.as_n_per_mm(),
            t.qy.as_n_per_mm(),
            1e-5 * q_scale,
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
    // 4 つの角の隅反力は等しい
    let r = s.corner_reaction(Corner::X0Y0).as_n();
    for c in Corner::ALL {
        assert_within(s.corner_reaction(c).as_n(), r, 1e-9 * r, &format!("{c:?}"));
    }
}

#[test]
fn poisson_ratio_scales_the_deflection_and_mixes_the_moments() {
    // 自由辺の無い型では無次元の解が ν に依らない（ADR-0005）: w ∝ 1/D ∝ 1 − ν²、Mx(ν) = Mx(0) + ν·My(0)。
    // 同じ級数の線形な組み合わせなので、差は丸めだけ（許容 1e-12）。辺の反力と隅反力は換算に乗らないので見ない
    let (lx, ly) = (2000.0, 2800.0);
    let nu = 0.3;
    let s0 = solve(&simply_supported(lx, ly, 0.0, 10.0)).unwrap();
    let s1 = solve(&simply_supported(lx, ly, nu, 10.0)).unwrap();
    for (x, y) in [(1000.0, 1400.0), (300.0, 1400.0), (600.0, 2500.0)] {
        let p0 = at(&s0, x, y);
        let p1 = at(&s1, x, y);
        assert_within(
            p1.w.as_mm(),
            (1.0 - nu * nu) * p0.w.as_mm(),
            1e-12 * p0.w.as_mm(),
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
        assert_within(
            p1.qx.as_n_per_mm(),
            p0.qx.as_n_per_mm(),
            1e-12 * p0.qx.as_n_per_mm().abs().max(1e-9),
            "Qx は ν に依らない",
        );
    }
}

/// 板の角の点か
fn at_corner(p: &PointResponse, lx: f64, ly: f64) -> bool {
    [0.0, lx].contains(&p.x.as_mm()) && [0.0, ly].contains(&p.y.as_mm())
}

/// 点の応答から 1 つの量を取り出す
type Pick = fn(&PointResponse) -> f64;

#[test]
fn doubling_the_terms_changes_nothing_beyond_the_tolerance() {
    // ADR-0006 の決め方を四辺単純支持にも当てる。縦横比 1・2・4 で、21×21 のグリッドと辺の 101 点で、
    // N = TERMS と 2N の差がその量の絶対値の最大の 1e-3 以下。グリッドは四辺固定と違って角の近くも収まり、除くのは
    // 角の点そのものだけ: そこではせん断力の厳密な値 0 に、内部の評価が選ぶ級数が三角関数の向きの端として 1/N で
    // しか近づかない（80 項で最大の 4e-3 程度）。辺の反力は角で 0 から急に立ち上がり、角から辺長の 5 % 以内では
    // 辺に沿う級数が振れる（縦横比 4 の長辺で最大の 1.4e-3）ので、四辺固定と同じくそこを除く
    let lx = 2000.0;
    for ratio in [1.0, 2.0, 4.0] {
        let ly = ratio * lx;
        let p = simply_supported(lx, ly, 0.2, 10.0);
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
            for (pa, pb) in ga.iter().zip(&gb).filter(|(p, _)| !at_corner(p, lx, ly)) {
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
            for (sa, sb) in ea.iter().zip(&eb).skip(5).take(91) {
                assert_within(
                    sa.reaction.as_n_per_mm(),
                    sb.reaction.as_n_per_mm(),
                    1e-3 * v_scale,
                    &format!("比 {ratio} の {e} の反力 {}", sa.along.as_mm()),
                );
                assert_within(
                    sa.shear.as_n_per_mm(),
                    sb.shear.as_n_per_mm(),
                    1e-3 * q_scale,
                    &format!("比 {ratio} の {e} のせん断力 {}", sa.along.as_mm()),
                );
            }
        }
        for c in Corner::ALL {
            let r = a.corner_reaction(c).as_n();
            assert_within(
                r,
                b.corner_reaction(c).as_n(),
                1e-3 * r,
                &format!("比 {ratio} の {c:?} の隅反力"),
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
    // 辺の上のせん断力は内部の評価と同じ値（辺の中央）。辺の評価は辺に沿う級数、内部の評価は辺を横切る級数を
    // 通り、後者の打ち切りは 1/N で減る（80 項で相対 2e-5 程度）。許容は 1e-4
    let edge = s.edge(Edge::X0, 2)[1];
    let point = at(&s, 0.0, l / 2.0);
    assert_within(
        edge.shear.as_n_per_mm(),
        point.qx.as_n_per_mm(),
        1e-4 * point.qx.as_n_per_mm().abs(),
        "辺の中央の Qx",
    );
}
