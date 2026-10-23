//! スパイク（計測専用）: 有限要素の連立を解く道を切り替える。`Band` が PR #65 の自前の帯行列
//! Cholesky、`Faer` と `Nalgebra` は feature で入る外部クレートの疎行列 Cholesky。

use super::band::BandMatrix;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearSolver {
    Band,
    #[cfg(feature = "faer")]
    Faer,
    #[cfg(feature = "nalgebra")]
    Nalgebra,
}

impl LinearSolver {
    pub const ALL: &[LinearSolver] = &[
        LinearSolver::Band,
        #[cfg(feature = "faer")]
        LinearSolver::Faer,
        #[cfg(feature = "nalgebra")]
        LinearSolver::Nalgebra,
    ];
}

/// 組み立てた対称正定値の連立。`entries` は下三角（row >= col）の要素で、同じ位置は何度も現れてよい
/// （足し合わせる）
#[derive(Debug, Clone)]
pub struct System {
    pub n: usize,
    pub bandwidth: usize,
    pub entries: Vec<(usize, usize, f64)>,
    pub rhs: Vec<f64>,
}

impl System {
    pub fn solve(self, solver: LinearSolver) -> Option<Vec<f64>> {
        match solver {
            LinearSolver::Band => self.solve_band(),
            #[cfg(feature = "faer")]
            LinearSolver::Faer => self.solve_faer(),
            #[cfg(feature = "nalgebra")]
            LinearSolver::Nalgebra => self.solve_nalgebra(),
        }
    }

    fn solve_band(self) -> Option<Vec<f64>> {
        let mut k = BandMatrix::zeros(self.n, self.bandwidth);
        for &(r, c, v) in &self.entries {
            k.add(r, c, v);
        }
        k.solve(self.rhs)
    }

    #[cfg(feature = "faer")]
    fn solve_faer(self) -> Option<Vec<f64>> {
        use faer::Side;
        use faer::prelude::Solve;
        use faer::sparse::{SparseColMat, Triplet};
        let triplets: Vec<Triplet<usize, usize, f64>> = self
            .entries
            .iter()
            .map(|&(r, c, v)| Triplet::new(r, c, v))
            .collect();
        let a =
            SparseColMat::<usize, f64>::try_new_from_triplets(self.n, self.n, &triplets).ok()?;
        let llt = a.sp_cholesky(Side::Lower).ok()?;
        let mut x = self.rhs;
        llt.solve_in_place(faer::ColMut::from_slice_mut(&mut x));
        Some(x)
    }

    #[cfg(feature = "nalgebra")]
    fn solve_nalgebra(self) -> Option<Vec<f64>> {
        use nalgebra_sparse::factorization::CscCholesky;
        use nalgebra_sparse::{CooMatrix, CscMatrix};
        let mut coo = CooMatrix::new(self.n, self.n);
        for &(r, c, v) in &self.entries {
            coo.push(r, c, v);
            if r != c {
                coo.push(c, r, v);
            }
        }
        let csc = CscMatrix::from(&coo);
        let chol = CscCholesky::factor(&csc).ok()?;
        let b = nalgebra::DMatrix::from_column_slice(self.n, 1, &self.rhs);
        let x = chol.solve(&b);
        Some(x.as_slice().to_vec())
    }
}
