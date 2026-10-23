//! 三辺固定一辺自由（FFFR）の Kirchhoff 板。入口 `solve` からだけ確かめる。期待値は理論から言える値（境界条件、
//! 帯板の極限、釣り合い、対称）と、項数を変えたときの収束だけで、文献の数値は置かない。
//! 代表の向きは x = 0・y = 0・x = Lx が固定、y = Ly が自由。

mod common;
use common::assert_within;
use rustruct_calc::kirchhoff_plate::{Plate, PointResponse, Solution, SupportType, solve};
use rustruct_types::kirchhoff_plate::{Edge, Support, Supports};
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

/// 代表の向き: y = Ly の辺が自由
fn free_at(edge: Edge) -> Supports {
    let mut s = Supports::all(Support::Fixed);
    match edge {
        Edge::X0 => s.x0 = Support::Free,
        Edge::Y0 => s.y0 = Support::Free,
        Edge::XLx => s.x_lx = Support::Free,
        Edge::YLy => s.y_ly = Support::Free,
    }
    s
}

fn fffr(lx: f64, ly: f64, nu: f64, p: f64) -> Plate {
    plate(lx, ly, nu, p, free_at(Edge::YLy))
}

fn at(s: &Solution, x: f64, y: f64) -> PointResponse {
    s.at(Length::mm(x), Length::mm(y)).expect("板の上の点")
}

#[test]
fn three_fixed_edges_and_one_free_edge_are_the_fffr_type() {
    for e in Edge::ALL {
        let s = solve(&plate(2000.0, 3000.0, 0.0, 10.0, free_at(e))).unwrap();
        assert_eq!(s.support_type(), SupportType::FFFR, "{e} が自由");
    }
}

#[test]
fn the_free_edge_has_zero_edge_moment_and_zero_edge_reaction() {
    for nu in [0.0, 0.2] {
        for (lx, ly) in [(2000.0, 2000.0), (2000.0, 1000.0), (2000.0, 4000.0)] {
            let s = solve(&fffr(lx, ly, nu, 10.0)).unwrap();
            let free = s.edge(Edge::YLy, 100);
            let fixed = s.edge(Edge::Y0, 100);
            let m_scale = fixed[50].moment.as_nmm_per_mm().abs();
            let v_scale = fixed[50].reaction.as_n_per_mm().abs();
            for (i, e) in free.iter().enumerate().skip(5).take(91) {
                assert_within(
                    e.moment.as_nmm_per_mm(),
                    0.0,
                    1e-3 * m_scale,
                    &format!("ν = {nu}、{lx}×{ly} の自由辺の My {i}"),
                );
                assert_within(
                    e.reaction.as_n_per_mm(),
                    0.0,
                    1e-3 * v_scale,
                    &format!("ν = {nu}、{lx}×{ly} の自由辺の反力 {i}"),
                );
            }
        }
    }
}

type Pick = fn(&PointResponse) -> f64;

#[test]
fn zz_dump() {
    use rustruct_calc::kirchhoff_plate::solve_with_terms;
    let lx = 2000.0;
    for ratio in [0.5, 1.0] {
        let ly = ratio * lx;
        let a = solve_with_terms(&fffr(lx, ly, 0.0, 10.0), 80).unwrap();
        let b = solve_with_terms(&fffr(lx, ly, 0.0, 10.0), 160).unwrap();
        for m in [0.05, 0.1, 0.15] {
            let mut line = format!("r{ratio} margin {m}:");
            for div in [100, 997] {
                let (ea, eb) = (a.edge(Edge::X0, div), b.edge(Edge::X0, div));
                let kept: Vec<_> = ea.iter().zip(&eb).filter(|(s, _)| { let t = s.along.as_mm() / ly; (m..=1.0 - m).contains(&t) }).collect();
                let all = ea.iter().map(|s| s.reaction.as_n_per_mm().abs()).fold(0.0, f64::max);
                let kmax = kept.iter().map(|(s, _)| s.reaction.as_n_per_mm().abs()).fold(0.0, f64::max);
                let d = kept.iter().map(|(p, q)| (p.reaction.as_n_per_mm() - q.reaction.as_n_per_mm()).abs()).fold(0.0, f64::max);
                line += &format!(" [{div}: V diff {d:.3} kN/m, /kept max {:.1e}, /all max {:.1e}]", d / kmax, d / all);
            }
            for div in [20, 50] {
                let (ga, gb) = (a.grid(div, div), b.grid(div, div));
                let keep = |p: &PointResponse| { let (x, y) = (p.x.as_mm() / lx, p.y.as_mm() / ly); (m..=1.0 - m).contains(&x) || (m..=1.0 - m).contains(&y) };
                for (name, f) in [("Qx", (|p: &PointResponse| p.qx.as_n_per_mm()) as Pick), ("Qy", |p| p.qy.as_n_per_mm())] {
                    let kmax = ga.iter().filter(|p| keep(p)).map(|p| f(p).abs()).fold(0.0, f64::max);
                    let mut worst = (0.0f64, 0.0, 0.0);
                    for (p, q) in ga.iter().zip(&gb).filter(|(p, _)| keep(p)) { let d = (f(p) - f(q)).abs() / kmax; if d > worst.0 { worst = (d, p.x.as_mm() / lx, p.y.as_mm() / ly); } }
                    line += &format!(" g{div} {name} {:.1e}@({:.2},{:.2})", worst.0, worst.1, worst.2);
                }
            }
            println!("{line}");
        }
    }
}
