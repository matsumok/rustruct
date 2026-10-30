//! 小さな密な連立一次方程式を部分ピボット付きの LU 分解（Gauss の消去法）で解く（サイト ADR-0009）。
//! 使い手は参照解（級数解）の未知の係数で、大きくても 100 元に届かない。

/// 行優先の正方行列。
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    n: usize,
    values: Vec<f64>,
}

impl Matrix {
    pub fn zeros(n: usize) -> Self {
        Self {
            n,
            values: vec![0.0; n * n],
        }
    }

    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.values[row * self.n + col]
    }

    pub fn set(&mut self, row: usize, col: usize, value: f64) {
        self.values[row * self.n + col] = value;
    }

    /// A x = b を解く。ピボットが 0 か極めて小さければ `None`（式か項数の取り方の誤り）
    pub fn solve(mut self, mut b: Vec<f64>) -> Option<Vec<f64>> {
        let n = self.n;
        assert_eq!(b.len(), n, "右辺の長さが行列の大きさと違う");
        // 各行の最大の絶対値。ピボットの大きさをこれと比べ、行の単位の取り方に依らず「極めて小さい」を判定する
        let mut scale: Vec<f64> = (0..n)
            .map(|r| (0..n).map(|c| self.get(r, c).abs()).fold(0.0, f64::max))
            .collect();
        for k in 0..n {
            // 部分ピボット: 列 k で、行の最大値に対する比が最も大きい行を選ぶ
            let pivot_row = (k..n)
                .max_by(|&r1, &r2| {
                    let v1 = self.get(r1, k).abs() / scale[r1];
                    let v2 = self.get(r2, k).abs() / scale[r2];
                    v1.total_cmp(&v2)
                })
                .expect("k < n");
            let pivot = self.get(pivot_row, k);
            // NaN は比較で偽になるので is_finite で拾う
            if pivot.abs() <= f64::EPSILON * scale[pivot_row] || !pivot.is_finite() {
                return None;
            }
            if pivot_row != k {
                for c in 0..n {
                    let t = self.get(k, c);
                    self.set(k, c, self.get(pivot_row, c));
                    self.set(pivot_row, c, t);
                }
                b.swap(k, pivot_row);
                scale.swap(k, pivot_row);
            }
            for r in (k + 1)..n {
                let factor = self.get(r, k) / pivot;
                if factor == 0.0 {
                    continue;
                }
                for c in k..n {
                    let v = self.get(r, c) - factor * self.get(k, c);
                    self.set(r, c, v);
                }
                b[r] -= factor * b[k];
            }
        }
        // 後退代入
        let mut x = vec![0.0; n];
        for k in (0..n).rev() {
            let sum: f64 = ((k + 1)..n).map(|c| self.get(k, c) * x[c]).sum();
            x[k] = (b[k] - sum) / self.get(k, k);
        }
        Some(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: &[&[f64]]) -> Matrix {
        let mut m = Matrix::zeros(rows.len());
        for (r, row) in rows.iter().enumerate() {
            for (c, &v) in row.iter().enumerate() {
                m.set(r, c, v);
            }
        }
        m
    }

    fn assert_vec(actual: &[f64], expected: &[f64]) {
        for (a, e) in actual.iter().zip(expected) {
            assert!(
                (a - e).abs() <= 1e-12 * e.abs().max(1.0),
                "{actual:?} != {expected:?}"
            );
        }
    }

    #[test]
    fn solves_a_small_system_with_a_known_solution() {
        // x = 1, y = 2, z = 3
        let a = matrix(&[&[2.0, 1.0, -1.0], &[-3.0, -1.0, 2.0], &[-2.0, 1.0, 2.0]]);
        let b = vec![1.0, 1.0, 6.0];
        assert_vec(&a.solve(b).unwrap(), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn pivots_when_the_leading_entry_is_zero() {
        // 左上が 0 なので行を入れ替えないと割れない。x = 1, y = 2
        let a = matrix(&[&[0.0, 1.0], &[1.0, 1.0]]);
        assert_vec(&a.solve(vec![2.0, 3.0]).unwrap(), &[1.0, 2.0]);
    }

    #[test]
    fn pivots_when_the_leading_entry_is_tiny() {
        // 先頭が 1e-20 のまま消去すると桁落ちで x が壊れる。x = 1, y = 1
        let a = matrix(&[&[1e-20, 1.0], &[1.0, 1.0]]);
        assert_vec(&a.solve(vec![1.0, 2.0]).unwrap(), &[1.0, 1.0]);
    }

    #[test]
    fn a_singular_system_is_reported() {
        let a = matrix(&[&[1.0, 2.0], &[2.0, 4.0]]);
        assert_eq!(a.solve(vec![1.0, 2.0]), None);
    }
}
