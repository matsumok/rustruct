//! 帯の色。正負のない量は `seq`、正負のある量は `div`（`docs/adr/0011`）。
//! どちらも tokens.json の 12 段で、段数が違えば 12 色の間を補間して取る。

use crate::scene::Paint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    /// 一方向（`seq-1`〜`seq-12`）。値が大きいほど地から遠い。
    Seq,
    /// 両方向（`div-1`〜`div-12`）。負が藍、正が琥珀、0 は `div-6` と `div-7` の境。
    Div,
}

impl Palette {
    /// 段数の目安の既定。たわみ（seq）は 10、曲げモーメントとせん断力（div）は 16（`docs/adr/0011`）。
    pub fn default_steps(self) -> usize {
        match self {
            Palette::Seq => 10,
            Palette::Div => 16,
        }
    }

    /// `n` 本の帯の `k` 番目（下から）の色。`div` は `n` が偶数で、`n / 2` 番目の境が 0 の前提。
    pub fn band_paint(self, k: usize, n: usize) -> Paint {
        match self {
            // 1 本なら最大側の色
            Palette::Seq => ramp("seq", 1, 12, position(k, n, 11.0, 11.0)),
            Palette::Div => {
                // 片側 1 本なら端の色（負は div-1、正は div-12）
                let half = n / 2;
                if k < half {
                    ramp("div", 1, 6, position(k, half, 5.0, 0.0))
                } else {
                    ramp("div", 7, 12, position(k - half, half, 5.0, 5.0))
                }
            }
        }
    }
}

/// `n` 本を `0`〜`stops` に等間隔に置いたときの `k` 番目の位置。`n` が 1 なら `single`。
fn position(k: usize, n: usize, stops: f64, single: f64) -> f64 {
    if n <= 1 {
        single
    } else {
        stops * k as f64 / (n - 1) as f64
    }
}

/// 役割名 `{prefix}-{first}`〜`{prefix}-{last}` の列の上の `position`（0 が `first`、`last - first` が `last`）の色。
fn ramp(prefix: &str, first: usize, last: usize, position: f64) -> Paint {
    let i = (position.floor() as usize).min(last - first - 1);
    Paint::Blend {
        from: format!("{prefix}-{}", first + i),
        to: format!("{prefix}-{}", first + i + 1),
        t: position - i as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blend(p: &Paint) -> (String, String, f64) {
        match p {
            Paint::Blend { from, to, t } => (from.clone(), to.clone(), *t),
            Paint::Role(r) => panic!("役割名そのもの: {r}"),
        }
    }

    #[test]
    fn twelve_seq_bands_use_the_twelve_stops() {
        for k in 0..12 {
            let (from, to, t) = blend(&Palette::Seq.band_paint(k, 12));
            let expected = if k < 11 {
                (format!("seq-{}", k + 1), format!("seq-{}", k + 2), 0.0)
            } else {
                ("seq-11".into(), "seq-12".into(), 1.0)
            };
            assert!(
                (t - expected.2).abs() < 1e-12 && from == expected.0 && to == expected.1,
                "{k}: {from} {to} {t}"
            );
        }
    }

    #[test]
    fn more_bands_interpolate_between_stops() {
        let (from, to, t) = blend(&Palette::Seq.band_paint(1, 16));
        assert_eq!((from.as_str(), to.as_str()), ("seq-1", "seq-2"));
        assert!((t - 11.0 / 15.0).abs() < 1e-12);
        let (from, to, _) = blend(&Palette::Seq.band_paint(15, 16));
        assert_eq!((from.as_str(), to.as_str()), ("seq-11", "seq-12"));
    }

    #[test]
    fn div_bands_split_at_zero() {
        let (from, _, t) = blend(&Palette::Div.band_paint(0, 12));
        assert_eq!((from.as_str(), t), ("div-1", 0.0));
        let (from, to, t) = blend(&Palette::Div.band_paint(5, 12));
        assert_eq!((from.as_str(), to.as_str(), t), ("div-5", "div-6", 1.0));
        let (from, _, t) = blend(&Palette::Div.band_paint(6, 12));
        assert_eq!((from.as_str(), t), ("div-7", 0.0));
        let (_, to, t) = blend(&Palette::Div.band_paint(11, 12));
        assert_eq!((to.as_str(), t), ("div-12", 1.0));
        // 2 本なら両端の色
        let (from, _, _) = blend(&Palette::Div.band_paint(0, 2));
        assert_eq!(from, "div-1");
        let (_, to, t) = blend(&Palette::Div.band_paint(1, 2));
        assert_eq!((to.as_str(), t), ("div-12", 1.0));
    }

    #[test]
    fn default_steps_follow_the_decision() {
        assert_eq!(Palette::Seq.default_steps(), 10);
        assert_eq!(Palette::Div.default_steps(), 16);
    }
}
