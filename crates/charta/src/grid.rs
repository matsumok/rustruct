//! 等間隔でなくてもよい直交グリッドのスカラー値。分布図は評価点のグリッドの値から描く。

/// `z` は行優先（`z[j * nx + i]`、`i` が x、`j` が y）。
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    xs: Vec<f64>,
    ys: Vec<f64>,
    z: Vec<f64>,
}

/// グリッドの形が合っていない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    /// x・y の座標は 2 つ以上要る（セルが無いと線も帯も引けない）。
    TooFew,
    /// `z` の長さが `xs.len() * ys.len()` でない。
    Length { expected: usize, got: usize },
}

impl Grid {
    /// 座標の列と行優先の値から。座標は昇順に並んでいること。
    pub fn new(xs: Vec<f64>, ys: Vec<f64>, z: Vec<f64>) -> Result<Self, GridError> {
        if xs.len() < 2 || ys.len() < 2 {
            return Err(GridError::TooFew);
        }
        let expected = xs.len() * ys.len();
        if z.len() != expected {
            return Err(GridError::Length {
                expected,
                got: z.len(),
            });
        }
        Ok(Self { xs, ys, z })
    }

    /// 関数を評価点で評価して作る。
    pub fn from_fn(
        xs: Vec<f64>,
        ys: Vec<f64>,
        f: impl Fn(f64, f64) -> f64,
    ) -> Result<Self, GridError> {
        let f = &f;
        let z = ys
            .iter()
            .flat_map(|&y| xs.iter().map(move |&x| f(x, y)))
            .collect();
        Self::new(xs, ys, z)
    }

    /// `[0, length]` を `n` 分割した `n + 1` 個の等間隔の座標。
    pub fn axis(length: f64, n: usize) -> Vec<f64> {
        (0..=n).map(|i| length * i as f64 / n as f64).collect()
    }

    pub fn xs(&self) -> &[f64] {
        &self.xs
    }

    pub fn ys(&self) -> &[f64] {
        &self.ys
    }

    pub fn at(&self, i: usize, j: usize) -> f64 {
        self.z[j * self.xs.len() + i]
    }

    /// 値の最小と最大。
    pub fn range(&self) -> (f64, f64) {
        self.z
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                (lo.min(v), hi.max(v))
            })
    }

    /// 座標の範囲 `(x0, x1, y0, y1)`。
    pub fn extent(&self) -> (f64, f64, f64, f64) {
        (
            self.xs[0],
            self.xs[self.xs.len() - 1],
            self.ys[0],
            self.ys[self.ys.len() - 1],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_is_checked() {
        assert_eq!(
            Grid::new(vec![0.0], vec![0.0, 1.0], vec![0.0, 0.0]),
            Err(GridError::TooFew)
        );
        assert_eq!(
            Grid::new(vec![0.0, 1.0], vec![0.0, 1.0], vec![0.0; 3]),
            Err(GridError::Length {
                expected: 4,
                got: 3
            })
        );
    }

    #[test]
    fn from_fn_is_row_major() {
        let g = Grid::from_fn(vec![0.0, 1.0, 2.0], vec![0.0, 10.0], |x, y| x + y).unwrap();
        assert_eq!(g.at(2, 0), 2.0);
        assert_eq!(g.at(1, 1), 11.0);
        assert_eq!(g.range(), (0.0, 12.0));
        assert_eq!(g.extent(), (0.0, 2.0, 0.0, 10.0));
        assert_eq!(Grid::axis(1.0, 4), vec![0.0, 0.25, 0.5, 0.75, 1.0]);
    }
}
