//! 対称で正定値な帯行列の連立一次方程式を Cholesky 分解で解く（サイト ADR-0009 の方針どおり、線形代数の
//! クレートは入れない）。使い手は有限要素の剛性行列で、節点の番号を短い辺に沿って振るので帯の幅は
//! 短い辺の節点数の数倍に収まる。

/// 下三角の帯だけを持つ対称行列。行 i は列 i − bw ..= i を持ち、それより左は 0。
#[derive(Debug, Clone, PartialEq)]
pub struct BandMatrix {
    n: usize,
    /// 半帯幅。対角を除いて左に持つ列の数
    bw: usize,
    /// 行ごとに bw + 1 個。行 i の列 j は `values[i * (bw + 1) + (j + bw − i)]`
    values: Vec<f64>,
}

impl BandMatrix {
    pub fn zeros(n: usize, bw: usize) -> Self {
        Self {
            n,
            bw,
            values: vec![0.0; n * (bw + 1)],
        }
    }

    fn index(&self, row: usize, col: usize) -> usize {
        debug_assert!(col <= row && row - col <= self.bw, "帯の外: ({row}, {col})");
        row * (self.bw + 1) + col + self.bw - row
    }

    /// 対称な位置 (row, col) と (col, row) に同じ値を足す。下三角の側だけを持つので一度でよい
    pub fn add(&mut self, row: usize, col: usize, value: f64) {
        let (row, col) = if col <= row { (row, col) } else { (col, row) };
        let i = self.index(row, col);
        self.values[i] += value;
    }

    /// A x = b を解く。A は正定値でなければならず、分解の途中で対角が正でなくなれば `None`
    /// （支持の足りない板は手前で弾くので、ここに来るのは拘束の組み立ての誤り）
    pub fn solve(mut self, mut b: Vec<f64>) -> Option<Vec<f64>> {
        let n = self.n;
        assert_eq!(b.len(), n, "右辺の長さが行列の大きさと違う");
        let bw = self.bw;
        // A = L Lᵀ。L を A と同じ帯に上書きする。行の帯は列について連続に並ぶので、内側の和は 2 つの
        // 並びの内積
        for j in 0..n {
            let first = j.saturating_sub(bw);
            let row_j = self.index(j, first)..self.index(j, j);
            let d = self.values[self.index(j, j)]
                - dot(&self.values[row_j.clone()], &self.values[row_j]);
            // NaN は比較で偽になるので、正で有限なときだけ進む
            if !(d.is_finite() && d > 0.0) {
                return None;
            }
            let ljj = d.sqrt();
            let jj = self.index(j, j);
            self.values[jj] = ljj;
            for i in (j + 1)..n.min(j + bw + 1) {
                let first = i.saturating_sub(bw).max(first);
                let row_i = self.index(i, first)..self.index(i, j);
                let row_j = self.index(j, first)..self.index(j, j);
                let s =
                    self.values[self.index(i, j)] - dot(&self.values[row_i], &self.values[row_j]);
                let idx = self.index(i, j);
                self.values[idx] = s / ljj;
            }
        }
        // L y = b
        for i in 0..n {
            let first = i.saturating_sub(bw);
            let row_i = self.index(i, first)..self.index(i, i);
            let s = b[i] - dot(&self.values[row_i], &b[first..i]);
            b[i] = s / self.values[self.index(i, i)];
        }
        // Lᵀ x = y。列 i は下の行に散らばるので、こちらは一つずつ
        for i in (0..n).rev() {
            let s = b[i]
                - ((i + 1)..n.min(i + bw + 1))
                    .map(|k| self.values[self.index(k, i)] * b[k])
                    .sum::<f64>();
            b[i] = s / self.values[self.index(i, i)];
        }
        Some(b)
    }
}

/// 内積。和を 4 つに分けて持ち、依存の鎖を短くする
fn dot(a: &[f64], b: &[f64]) -> f64 {
    let mut acc = [0.0; 4];
    let (a4, a_rest) = a.as_chunks::<4>();
    let (b4, b_rest) = b.as_chunks::<4>();
    for (x, y) in a4.iter().zip(b4) {
        for ((acc, x), y) in acc.iter_mut().zip(x).zip(y) {
            *acc += x * y;
        }
    }
    let tail: f64 = a_rest.iter().zip(b_rest).map(|(x, y)| x * y).sum();
    acc[0] + acc[1] + acc[2] + acc[3] + tail
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_vec(actual: &[f64], expected: &[f64]) {
        for (a, e) in actual.iter().zip(expected) {
            assert!(
                (a - e).abs() <= 1e-12 * e.abs().max(1.0),
                "{actual:?} != {expected:?}"
            );
        }
    }

    #[test]
    fn solves_a_tridiagonal_system_with_a_known_solution() {
        // 対角 2、隣 −1 の 4 元。x = (1, 2, 3, 4) のとき b = (0, 0, 0, 5)
        let mut a = BandMatrix::zeros(4, 1);
        for i in 0..4 {
            a.add(i, i, 2.0);
            if i > 0 {
                a.add(i, i - 1, -1.0);
            }
        }
        assert_vec(
            &a.solve(vec![0.0, 0.0, 0.0, 5.0]).unwrap(),
            &[1.0, 2.0, 3.0, 4.0],
        );
    }

    #[test]
    fn a_wider_band_entered_from_either_triangle_gives_the_same_matrix() {
        // 帯幅 2 の 5 元。上三角の側から足しても同じ行列になる。x = (1, −1, 2, 0, 3)
        let rows: [[f64; 5]; 5] = [
            [6.0, 1.0, 2.0, 0.0, 0.0],
            [1.0, 7.0, 1.0, 2.0, 0.0],
            [2.0, 1.0, 8.0, 1.0, 2.0],
            [0.0, 2.0, 1.0, 9.0, 1.0],
            [0.0, 0.0, 2.0, 1.0, 10.0],
        ];
        let x = [1.0, -1.0, 2.0, 0.0, 3.0];
        let b: Vec<f64> = rows
            .iter()
            .map(|r| r.iter().zip(&x).map(|(a, x)| a * x).sum())
            .collect();
        let mut lower = BandMatrix::zeros(5, 2);
        let mut upper = BandMatrix::zeros(5, 2);
        for (i, row) in rows.iter().enumerate() {
            for (j, &value) in row.iter().enumerate().take(i + 1) {
                if value != 0.0 {
                    lower.add(i, j, value);
                    upper.add(j, i, value);
                }
            }
        }
        assert_eq!(lower, upper);
        assert_vec(&lower.solve(b).unwrap(), &x);
    }

    #[test]
    fn a_matrix_that_is_not_positive_definite_is_reported() {
        let mut a = BandMatrix::zeros(2, 1);
        a.add(0, 0, 1.0);
        a.add(1, 0, 2.0);
        a.add(1, 1, 1.0);
        assert_eq!(a.solve(vec![1.0, 1.0]), None);
    }
}
