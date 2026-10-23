//! Kirchhoff 板の有限要素の解き方。入口 `solve_with_method` からだけ確かめる。期待値は理論から言える値
//! （釣り合い、対称、帯板の極限、Navier の二重級数）と、同じ板の級数解、分割数を倍にしたときの収束だけ。

mod common;
use std::f64::consts::PI;

use common::assert_within;
use rustruct_calc::kirchhoff_plate::{
    DIVISIONS, EdgeSample, Method, Plate, PlateError, PointResponse, Solution, SupportType,
    mesh_divisions, solve, solve_with_divisions, solve_with_method,
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

const SSSS: Supports = Supports::all(Support::SimplySupported);
const FFFF: Supports = Supports::all(Support::Fixed);
/// 三辺固定一辺自由。自由辺は y = Ly
const FFFR: Supports = Supports {
    x0: Support::Fixed,
    y0: Support::Fixed,
    x_lx: Support::Fixed,
    y_ly: Support::Free,
};

/// 曲げ剛性 D = E t³ / (12 (1 − ν²)) [N·mm]
fn rigidity(plate: &Plate) -> f64 {
    let t = plate.thickness.as_mm();
    plate.young_modulus.as_n_per_mm2() * t * t * t
        / (12.0 * (1.0 - plate.poisson_ratio.value().powi(2)))
}

fn fe(plate: &Plate) -> Solution {
    solve_with_method(plate, Method::FiniteElement).unwrap()
}

fn at(s: &Solution, x: f64, y: f64) -> PointResponse {
    s.at(Length::mm(x), Length::mm(y)).expect("板の上の点")
}

/// 辺の反力の合計 − 隅反力の合計 [N]
fn reactions_minus_corners(s: &Solution) -> f64 {
    let reactions: f64 = Edge::ALL.iter().map(|&e| s.total_reaction(e).as_n()).sum();
    let corners: f64 = Corner::ALL
        .iter()
        .map(|&c| s.corner_reaction(c).as_n())
        .sum();
    reactions - corners
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

/// 隅から辺長の 5 % 以内の点か（x・y ともに辺長の 5 % より端に寄っている）
fn near_corner(x: f64, y: f64, lx: f64, ly: f64) -> bool {
    !(0.05 * lx..=0.95 * lx).contains(&x) && !(0.05 * ly..=0.95 * ly).contains(&y)
}

/// 点の応答から 1 つの量を取り出す
type Pick = fn(&PointResponse) -> f64;

const POINT_QUANTITIES: [(&str, Pick); 5] = [
    ("w", |p| p.w.as_mm()),
    ("Mx", |p| p.mx.as_nmm_per_mm()),
    ("My", |p| p.my.as_nmm_per_mm()),
    ("Qx", |p| p.qx.as_n_per_mm()),
    ("Qy", |p| p.qy.as_n_per_mm()),
];

type PickEdge = fn(&EdgeSample) -> f64;

const EDGE_QUANTITIES: [(&str, PickEdge); 3] = [
    ("M", |s| s.moment.as_nmm_per_mm()),
    ("Q", |s| s.shear.as_n_per_mm()),
    ("V", |s| s.reaction.as_n_per_mm()),
];

/// たわみと曲げモーメントは 0.5 %、せん断力と辺の反力は 2 %（その量の絶対値の最大に対して）
fn tolerance_of(name: &str) -> f64 {
    if name.starts_with('w') || name.starts_with('M') {
        5e-3
    } else {
        2e-2
    }
}

/// 2 つの解の差。量ごとに、基準の解のその量の絶対値の最大に対する比の最大と、その点
struct Deviation {
    name: String,
    ratio: f64,
    at: String,
}

/// 21 × 21 のグリッド（隅から辺長の 5 % 以内を除く）と、各辺の 101 点（両端から 5 % を除く）、隅反力で比べる。
/// 比の分母は量の種類ごと（たわみ、曲げモーメント、せん断力、辺の反力、隅反力）に、基準の解の板全体での
/// 絶対値の最大。単純支持・自由の辺の曲げモーメントや自由辺の反力は厳密に 0 で、板全体の最大と比べる
fn deviations(reference: &Solution, other: &Solution) -> Vec<Deviation> {
    let (lx, ly) = (reference.plate().lx.as_mm(), reference.plate().ly.as_mm());
    let mut out = Vec::new();
    let (ga, gb) = (reference.grid(20, 20), other.grid(20, 20));
    let edges: Vec<(Edge, Vec<EdgeSample>, Vec<EdgeSample>)> = Edge::ALL
        .iter()
        .map(|&e| (e, reference.edge(e, 100), other.edge(e, 100)))
        .collect();
    let max = |values: &mut dyn Iterator<Item = f64>| values.map(f64::abs).fold(0.0, f64::max);
    let w_scale = max(&mut ga.iter().map(|p| p.w.as_mm()));
    let m_scale = max(&mut ga
        .iter()
        .flat_map(|p| [p.mx.as_nmm_per_mm(), p.my.as_nmm_per_mm()]));
    let q_scale = max(&mut ga
        .iter()
        .flat_map(|p| [p.qx.as_n_per_mm(), p.qy.as_n_per_mm()])
        .chain(
            edges
                .iter()
                .flat_map(|(_, ea, _)| ea.iter().map(|s| s.shear.as_n_per_mm())),
        ));
    let v_scale = max(&mut edges
        .iter()
        .flat_map(|(_, ea, _)| ea.iter().map(|s| s.reaction.as_n_per_mm())));
    for (name, f) in POINT_QUANTITIES {
        let scale = match name {
            "w" => w_scale,
            "Mx" | "My" => m_scale,
            _ => q_scale,
        };
        let mut worst = Deviation {
            name: name.to_string(),
            ratio: 0.0,
            at: String::new(),
        };
        for (pa, pb) in ga.iter().zip(&gb) {
            let (x, y) = (pa.x.as_mm(), pa.y.as_mm());
            if near_corner(x, y, lx, ly) {
                continue;
            }
            let r = (f(pa) - f(pb)).abs() / scale;
            if r > worst.ratio {
                worst.ratio = r;
                worst.at = format!("({x}, {y})");
            }
        }
        out.push(worst);
    }
    for (e, ea, eb) in &edges {
        for (name, f) in EDGE_QUANTITIES {
            let scale = match name {
                "M" => m_scale,
                "Q" => q_scale,
                _ => v_scale,
            };
            let mut worst = Deviation {
                name: format!("{e} の {name}"),
                ratio: 0.0,
                at: String::new(),
            };
            for (sa, sb) in ea.iter().zip(eb).skip(5).take(91) {
                let r = (f(sa) - f(sb)).abs() / scale;
                if r > worst.ratio {
                    worst.ratio = r;
                    worst.at = format!("{}", sa.along.as_mm());
                }
            }
            out.push(worst);
        }
    }
    let scale = max(&mut Corner::ALL
        .iter()
        .map(|&c| reference.corner_reaction(c).as_n()));
    if scale > 0.0 {
        for c in Corner::ALL {
            let r = (reference.corner_reaction(c).as_n() - other.corner_reaction(c).as_n()).abs()
                / scale;
            out.push(Deviation {
                name: "隅反力".to_string(),
                ratio: r,
                at: format!("{c:?}"),
            });
        }
    }
    out
}

fn assert_deviations_within_tolerance(reference: &Solution, other: &Solution, what: &str) {
    for d in deviations(reference, other) {
        let tolerance = tolerance_of(&d.name);
        assert!(
            d.ratio <= tolerance,
            "{what} の {}: 差 {:.2e} が許容 {tolerance} を超える（{}）",
            d.name,
            d.ratio,
            d.at
        );
    }
}

#[test]
fn unstable_types_are_refused_and_the_other_nineteen_solve() {
    for ty in SupportType::ALL {
        let p = plate(2000.0, 3000.0, 0.2, 10.0, ty.representative());
        let result = solve_with_method(&p, Method::FiniteElement);
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
fn the_series_solvers_are_the_default_and_still_refuse_types_without_a_series() {
    let p = plate(2000.0, 2000.0, 0.0, 10.0, FFFR);
    assert_eq!(
        solve(&p).err(),
        Some(PlateError::Unsupported(SupportType::FFFR))
    );
    assert_eq!(
        solve_with_method(&p, Method::Series).err(),
        Some(PlateError::Unsupported(SupportType::FFFR))
    );
    assert!(solve_with_method(&p, Method::FiniteElement).is_ok());
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

struct Navier {
    w: f64,
    mx: f64,
    my: f64,
    qx: f64,
    qy: f64,
    w_xy: f64,
}

/// 四辺単純支持の Navier の二重正弦級数。有限要素と独立な解
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
fn the_simply_supported_plate_matches_the_navier_double_series() {
    // Navier の打ち切りの差は各量の最大の 1e-5 未満（m・n を 799 まで）なので、差は有限要素の離散化の誤差。
    // 許容はたわみと曲げモーメントが各量の代表値の 0.5 %、せん断力が 2 %、隅反力が 2 %
    let (lx, ly) = (2000.0, 3000.0);
    let p = plate(lx, ly, 0.2, 10.0, SSSS);
    let s = fe(&p);
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
            5e-3 * centre.w,
            &format!("{what} の w"),
        );
        assert_within(
            ours.mx.as_nmm_per_mm(),
            n.mx,
            5e-3 * centre.mx,
            &format!("{what} の Mx"),
        );
        assert_within(
            ours.my.as_nmm_per_mm(),
            n.my,
            5e-3 * centre.mx,
            &format!("{what} の My"),
        );
        assert_within(
            ours.qx.as_n_per_mm(),
            n.qx,
            2e-2 * q_scale,
            &format!("{what} の Qx"),
        );
        assert_within(
            ours.qy.as_n_per_mm(),
            n.qy,
            2e-2 * q_scale,
            &format!("{what} の Qy"),
        );
    }
    let expected = 2.0 * rigidity(&p) * (1.0 - 0.2) * navier(&p, 0.0, 0.0, 799).w_xy;
    for c in Corner::ALL {
        assert_within(
            s.corner_reaction(c).as_n(),
            expected,
            2e-2 * expected,
            &format!("{c:?} の隅反力"),
        );
    }
}

#[test]
fn the_simply_supported_and_fixed_plates_match_the_series_solutions() {
    // 四辺単純支持と四辺固定は級数解がある。縦横比 1・2・4、ν = 0 と 0.2 で、隅から辺長の 5 % を除いた
    // 範囲の w・Mx・My が 0.5 % 以内、Qx・Qy・辺の反力が 2 % 以内（その量の絶対値の最大に対して）
    let lx = 2000.0;
    for (symbol, supports) in [("SSSS", SSSS), ("FFFF", FFFF)] {
        for nu in [0.0, 0.2] {
            for ratio in [1.0, 2.0, 4.0] {
                let p = plate(lx, ratio * lx, nu, 10.0, supports);
                let series = solve(&p).unwrap();
                assert_deviations_within_tolerance(
                    &series,
                    &fe(&p),
                    &format!("{symbol} ν = {nu} 比 {ratio}"),
                );
            }
        }
    }
}

#[test]
fn a_long_three_fixed_one_free_plate_behaves_as_a_fixed_strip_across_the_short_span() {
    // 三辺固定一辺自由で Ly/Lx = 6、ν = 0。y = 0 の固定辺から離れた断面は両端固定の梁で、自由辺（y = Ly）の
    // 上でもそのまま（ν = 0 なら帯板の解が自由辺の条件を満たす）。中央 pL²/24、固定辺 −pL²/12、
    // たわみ pL⁴/(384D)、固定辺のせん断力と反力 pL/2。許容はたわみと曲げモーメントが 0.5 %、
    // せん断力と反力が 2 %
    let lx = 2000.0;
    let p = plate(lx, 6.0 * lx, 0.0, 10.0, FFFR);
    let s = fe(&p);
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
fn edge_reactions_minus_corner_reactions_balance_the_total_load_to_round_off() {
    // 辺の反力の合計は節点の反力の和そのもので、隅反力を引くと荷重の合計に丸め誤差（連立の大きさで
    // 1e-10 の桁。許容 1e-8）で一致する（ADR-0004）。固定辺を含む隅と自由辺どうしの隅の隅反力は 0
    let frrr = Supports {
        x0: Support::Fixed,
        y0: Support::Free,
        x_lx: Support::Free,
        y_ly: Support::Free,
    };
    let rsrs = Supports {
        x0: Support::Free,
        y0: Support::SimplySupported,
        x_lx: Support::Free,
        y_ly: Support::SimplySupported,
    };
    let fsrs = Supports {
        x0: Support::Fixed,
        y0: Support::SimplySupported,
        x_lx: Support::Free,
        y_ly: Support::SimplySupported,
    };
    for supports in [SSSS, FFFF, FFFR, frrr, rsrs, fsrs] {
        for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (4000.0, 2000.0)] {
            let p = plate(lx, ly, 0.15, 8.0, supports);
            let s = fe(&p);
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
        }
    }
    // 四辺単純支持では隅反力があるので、辺の反力の合計は荷重の合計より大きい
    let p = plate(2000.0, 3000.0, 0.15, 8.0, SSSS);
    let s = fe(&p);
    let reactions: f64 = Edge::ALL.iter().map(|&e| s.total_reaction(e).as_n()).sum();
    assert!(reactions > p.load.as_n_per_mm2() * 2000.0 * 3000.0);
    for c in Corner::ALL {
        assert!(s.corner_reaction(c).as_n() > 0.0);
    }
}

#[test]
fn the_edge_samples_integrate_to_the_total_reaction() {
    // 辺の反力の分布は節点の間で線形なので、細かい台形則の積分は合計に 1e-4 で一致する（節点の位置と評価点が
    // 揃わない分だけ差が出る）。級数解の合計も同じ台形則で、辺の反力の分布と食い違わない
    let p = plate(2000.0, 3000.0, 0.2, 10.0, FFFF);
    for s in [fe(&p), solve(&p).unwrap()] {
        for e in Edge::ALL {
            let samples = s.edge(e, 2000);
            let trapezoid: f64 = samples
                .windows(2)
                .map(|pair| {
                    let ds = pair[1].along.as_mm() - pair[0].along.as_mm();
                    (pair[0].reaction.as_n_per_mm() + pair[1].reaction.as_n_per_mm()) / 2.0 * ds
                })
                .sum();
            let total = s.total_reaction(e).as_n();
            assert_within(trapezoid, total, 1e-4 * total.abs(), &format!("{e} の合計"));
        }
    }
}

#[test]
fn swapping_lx_and_ly_swaps_x_and_y_in_the_results() {
    // 2000×3000 の (x, y) と 3000×2000 の (y, x) は同じ板。要素の分割も入れ替わるだけなので、差は丸め誤差
    let (lx, ly) = (2000.0, 3000.0);
    let transposed = |s: Supports| Supports {
        x0: s.y0,
        y0: s.x0,
        x_lx: s.y_ly,
        y_ly: s.x_lx,
    };
    for supports in [FFFF, FFFR, SSSS] {
        let a = fe(&plate(lx, ly, 0.2, 10.0, supports));
        let b = fe(&plate(ly, lx, 0.2, 10.0, transposed(supports)));
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
fn a_square_plate_is_symmetric_under_the_symmetries_of_the_square() {
    // 四辺同じ支持の正方形は、x と y の入れ替え、x の裏返し、y の裏返しで同じ板。要素の分割も対称なので、
    // 差は丸め誤差。せん断力は裏返す軸の向きで符号が変わる
    let l = 2000.0;
    for supports in [SSSS, FFFF] {
        let s = fe(&plate(l, l, 0.2, 10.0, supports));
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
            (300.0, 700.0),
            (1000.0, 1500.0),
            (100.0, 1900.0),
            (0.0, 1200.0),
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
        for c in Corner::ALL {
            close(
                s.corner_reaction(c).as_n(),
                s.corner_reaction(Corner::X0Y0).as_n(),
                "隅反力",
            );
        }
    }
    // 三辺固定一辺自由は x の裏返しだけ
    let s = fe(&plate(l, l, 0.2, 10.0, FFFR));
    let scale = Scale::of(&s);
    for (x, y) in [(300.0, 700.0), (100.0, 1900.0), (0.0, 1200.0)] {
        let (r, fx) = (at(&s, x, y), at(&s, l - x, y));
        assert_within(
            r.w.as_mm(),
            fx.w.as_mm(),
            1e-9 * scale.w,
            "FFFR の x の裏返しの w",
        );
        assert_within(
            r.qx.as_n_per_mm(),
            -fx.qx.as_n_per_mm(),
            1e-9 * scale.q,
            "FFFR の x の裏返しの Qx",
        );
    }
}

#[test]
fn doubling_the_divisions_changes_nothing_beyond_the_tolerance() {
    // 分割数を決めた根拠（ADR-0006 の項数と同じ位置づけ）。縦横比 1・2・4、ν = 0.2 で、短い辺の分割数を
    // DIVISIONS から倍にしたときの差が、級数解との照合と同じ許容（w・M は 0.5 %、Q・V は 2 %）に収まる
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
fn the_shear_and_reaction_along_a_fixed_edge_are_smooth_when_nu_is_not_zero() {
    // 自由辺のある型を ν ≠ 0 で解いても、固定辺のせん断力と反力は辺に沿って振れない。両端から 5 % を除いた
    // 範囲で、隣り合う評価点の差の符号が変わるのは 1 回以下（辺の中央で向きが変わるだけ）
    let p = plate(2000.0, 2000.0, 0.2, 10.0, FFFR);
    let s = fe(&p);
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

#[test]
#[ignore = "級数解との差の表と計算時間を出す。cargo test --release -- --ignored --nocapture で見る"]
fn report_deviations_and_timing() {
    let lx = 2000.0;
    for (symbol, supports) in [("SSSS", SSSS), ("FFFF", FFFF), ("FFFR", FFFR)] {
        for nu in [0.0, 0.2] {
            for ratio in [1.0, 2.0, 4.0] {
                let p = plate(lx, ratio * lx, nu, 10.0, supports);
                let t = std::time::Instant::now();
                let f = fe(&p);
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
                if let Ok(series) = solve(&p) {
                    for d in deviations(&series, &f) {
                        println!("  {:<14} {:.2e} at {}", d.name, d.ratio, d.at);
                    }
                }
            }
        }
    }
}
