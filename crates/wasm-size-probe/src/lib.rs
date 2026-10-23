//! スパイク（計測専用）。`probe_assemble(ratio)` で連立を組み立てて持ち、`probe_solve(solver)` で
//! `LinearSolver::ALL[solver]` を使って解く。戻り値は解の和（最適化で消えないように）

use std::cell::RefCell;

use rustruct_calc::kirchhoff_plate::linear_solver::System;
use rustruct_calc::kirchhoff_plate::{DIVISIONS, LinearSolver, assemble};
use rustruct_types::kirchhoff_plate::{Support, Supports};

thread_local! {
    static SYSTEM: RefCell<Option<System>> = const { RefCell::new(None) };
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_assemble(ratio: f64, fixed: u32) -> u32 {
    let supports = if fixed != 0 {
        Supports::all(Support::Fixed)
    } else {
        Supports::all(Support::SimplySupported)
    };
    let assembled = assemble(ratio, 0.0, supports, DIVISIONS);
    let n = assembled.system.n;
    SYSTEM.with(|s| *s.borrow_mut() = Some(assembled.system));
    n as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_solve(solver: u32) -> f64 {
    let system = SYSTEM
        .with(|s| s.borrow().clone())
        .expect("probe_assemble first");
    let x = system
        .solve(LinearSolver::ALL[solver as usize])
        .expect("solvable");
    x.iter().sum()
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_solver_count() -> u32 {
    LinearSolver::ALL.len() as u32
}
