//! スパイク（計測専用）: 連立の解き方ごとの一致と時間。`cargo run --release -p rustruct-calc
//! --example spike_solver --features faer,nalgebra`

use std::time::{Duration, Instant};

use rustruct_calc::kirchhoff_plate::{DIVISIONS, LinearSolver, assemble};
use rustruct_types::kirchhoff_plate::{Support, Supports};

const RUNS: usize = 10;

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn main() {
    let cases = [
        ("SSSS", Supports::all(Support::SimplySupported), 1.0),
        ("SSSS", Supports::all(Support::SimplySupported), 4.0),
        ("FFFF", Supports::all(Support::Fixed), 1.0),
        ("FFFF", Supports::all(Support::Fixed), 4.0),
    ];
    for (name, supports, b) in cases {
        let nu = 0.0;
        let assembled = assemble(b, nu, supports, DIVISIONS);
        let sys = &assembled.system;
        println!(
            "== {name} b={b}: n={} bandwidth={} entries={}",
            sys.n,
            sys.bandwidth,
            sys.entries.len()
        );
        let reference = sys.clone().solve(LinearSolver::Band).unwrap();
        for &solver in LinearSolver::ALL {
            let x = sys.clone().solve(solver).unwrap();
            let max_abs = x
                .iter()
                .zip(&reference)
                .map(|(a, r)| (a - r).abs())
                .fold(0.0, f64::max);
            let max_ref = reference.iter().fold(0.0_f64, |m, r| m.max(r.abs()));
            let mut solve_only = Vec::new();
            let mut total = Vec::new();
            for _ in 0..RUNS {
                let t0 = Instant::now();
                let a = assemble(b, nu, supports, DIVISIONS);
                let t1 = Instant::now();
                let _ = a.system.solve(solver).unwrap();
                let t2 = Instant::now();
                solve_only.push(t2 - t1);
                total.push(t2 - t0);
            }
            println!(
                "{name} b={b} {solver:?}: max|dx|={max_abs:.3e} (max|x|={max_ref:.3e}) solve={:.2?} assemble+solve={:.2?}",
                median(solve_only),
                median(total)
            );
        }
    }
}
