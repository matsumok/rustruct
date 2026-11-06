//! 帯の色。正負のない量は `seq`、正負のある量は `div`（ADR site-0011）。
//! どちらも [`Theme`] にある色の列（`seq-1`、`seq-2` …）の段で、段数が違えば列の間を補間して取る。
//! `div` の列は前半が負、後半が正。

use crate::Error;
use crate::scene::Paint;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    /// 一方向（`seq-1`〜）。値が大きいほど地から遠い。
    Seq,
    /// 両方向（`div-1`〜）。負が藍、正が琥珀、0 は列の前半と後半の境。
    Div,
}

impl Palette {
    /// 段数の目安の既定。たわみ（seq）は 10、曲げモーメントとせん断力（div）は 16（ADR site-0011）。
    pub fn default_steps(self) -> usize {
        match self {
            Palette::Seq => 10,
            Palette::Div => 16,
        }
    }

    /// 色の列の役割名の頭。
    pub fn prefix(self) -> &'static str {
        match self {
            Palette::Seq => "seq",
            Palette::Div => "div",
        }
    }

    /// `n` 本の帯の `k` 番目（下から）の色。段の色は描くときに [`Palette::blend`] で決まる。
    pub fn band_paint(self, k: usize, n: usize) -> Paint {
        Paint::Band {
            palette: self,
            k,
            n,
        }
    }

    /// `n` 本の帯の `k` 番目（下から）の色を、`theme` の列の隣り合う 2 段の間の色にする。
    /// `div` は `n` が偶数で、`n / 2` 番目の境が 0 の前提。列の段数は `seq` が 2 以上、`div` が 4 以上の偶数。
    pub fn blend(self, k: usize, n: usize, theme: &Theme) -> Result<Paint, Error> {
        let prefix = self.prefix();
        let stops = theme.stops(prefix);
        match self {
            Palette::Seq if stops >= 2 => {
                // 1 本なら最大側の色
                let last = (stops - 1) as f64;
                Ok(ramp(prefix, 1, stops, position(k, n, last, last)))
            }
            Palette::Div if stops >= 4 && stops.is_multiple_of(2) => {
                // 片側 1 本なら端の色（負は最初の段、正は最後の段）
                let (half, side) = (n / 2, stops / 2);
                let last = (side - 1) as f64;
                Ok(if k < half {
                    ramp(prefix, 1, side, position(k, half, last, 0.0))
                } else {
                    ramp(
                        prefix,
                        side + 1,
                        stops,
                        position(k - half, half, last, last),
                    )
                })
            }
            _ => Err(Error::Stops {
                prefix: prefix.to_string(),
                count: stops,
            }),
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

/// 継ぎ目: 帯の番号と本数 → 列の 2 段と間の位置。列の段数は渡した `Theme` から数える。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::sample;

    fn blend(palette: Palette, k: usize, n: usize, theme: &Theme) -> (String, String, f64) {
        match palette.blend(k, n, theme).unwrap() {
            Paint::Blend { from, to, t } => (from, to, t),
            p => panic!("段の間の色でない: {p:?}"),
        }
    }

    /// `seq` と `div` を `stops` 段ずつ持つ表。
    fn with_stops(stops: usize) -> Theme {
        let colors = (1..=stops).flat_map(|k| {
            [
                (format!("seq-{k}"), "#000000".to_string()),
                (format!("div-{k}"), "#000000".to_string()),
            ]
        });
        Theme::new(colors, "", "")
    }

    #[test]
    fn twelve_seq_bands_use_the_twelve_stops() {
        let t = sample();
        for k in 0..12 {
            let (from, to, t) = blend(Palette::Seq, k, 12, &t);
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
        let t = sample();
        let (from, to, t1) = blend(Palette::Seq, 1, 16, &t);
        assert_eq!((from.as_str(), to.as_str()), ("seq-1", "seq-2"));
        assert!((t1 - 11.0 / 15.0).abs() < 1e-12);
        let (from, to, _) = blend(Palette::Seq, 15, 16, &t);
        assert_eq!((from.as_str(), to.as_str()), ("seq-11", "seq-12"));
    }

    #[test]
    fn div_bands_split_at_zero() {
        let t = sample();
        let (from, _, t1) = blend(Palette::Div, 0, 12, &t);
        assert_eq!((from.as_str(), t1), ("div-1", 0.0));
        let (from, to, t1) = blend(Palette::Div, 5, 12, &t);
        assert_eq!((from.as_str(), to.as_str(), t1), ("div-5", "div-6", 1.0));
        let (from, _, t1) = blend(Palette::Div, 6, 12, &t);
        assert_eq!((from.as_str(), t1), ("div-7", 0.0));
        let (_, to, t1) = blend(Palette::Div, 11, 12, &t);
        assert_eq!((to.as_str(), t1), ("div-12", 1.0));
        // 2 本なら両端の色
        let (from, _, _) = blend(Palette::Div, 0, 2, &t);
        assert_eq!(from, "div-1");
        let (_, to, t1) = blend(Palette::Div, 1, 2, &t);
        assert_eq!((to.as_str(), t1), ("div-12", 1.0));
    }

    #[test]
    fn the_number_of_stops_comes_from_the_theme() {
        let t = with_stops(4);
        // seq 4 段に 4 本: 段ごと。最後は seq-3 と seq-4 の間の端
        let (from, to, t1) = blend(Palette::Seq, 3, 4, &t);
        assert_eq!((from.as_str(), to.as_str(), t1), ("seq-3", "seq-4", 1.0));
        let (from, _, t1) = blend(Palette::Seq, 1, 4, &t);
        assert_eq!((from.as_str(), t1), ("seq-2", 0.0));
        // div 4 段: 負は div-1〜2、正は div-3〜4
        let (from, to, t1) = blend(Palette::Div, 1, 4, &t);
        assert_eq!((from.as_str(), to.as_str(), t1), ("div-1", "div-2", 1.0));
        let (from, _, t1) = blend(Palette::Div, 2, 4, &t);
        assert_eq!((from.as_str(), t1), ("div-3", 0.0));
    }

    #[test]
    fn too_few_or_odd_stops_are_an_error() {
        assert_eq!(
            Palette::Seq.blend(0, 4, &with_stops(1)),
            Err(Error::Stops {
                prefix: "seq".into(),
                count: 1
            })
        );
        assert_eq!(
            Palette::Div.blend(0, 4, &with_stops(5)),
            Err(Error::Stops {
                prefix: "div".into(),
                count: 5
            })
        );
    }

    #[test]
    fn band_paint_defers_the_stops_to_drawing() {
        assert_eq!(
            Palette::Div.band_paint(3, 10),
            Paint::Band {
                palette: Palette::Div,
                k: 3,
                n: 10
            }
        );
    }

    #[test]
    fn default_steps_follow_the_decision() {
        assert_eq!(Palette::Seq.default_steps(), 10);
        assert_eq!(Palette::Div.default_steps(), 16);
    }
}
