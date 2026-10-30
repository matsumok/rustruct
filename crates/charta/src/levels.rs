//! 値の範囲と段数の目安から帯の境を決める。刻みは 1・2・2.5・5 × 10ⁿ に丸める（`docs/adr/0011`）。

use crate::palette::Palette;

/// `span` を `steps` 個に割った刻みを、1・2・2.5・5 × 10ⁿ のうち割った値以上で最小のものに丸める。
/// `span` が正でなければ、`span` を 1 とみなす（平らな場でも帯が 1 本できる）。
pub fn nice_step(span: f64, steps: usize) -> f64 {
    let span = if span > 0.0 && span.is_finite() {
        span
    } else {
        1.0
    };
    let raw = span / steps.max(1) as f64;
    let decade = 10f64.powi(raw.log10().floor() as i32);
    [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .map(|m| m * decade)
        .find(|&s| s >= raw * (1.0 - 1e-9))
        .expect("10 × decade は raw 以上")
}

/// 帯の境（昇順、`steps` 本前後の帯）。`seq` は最小から最大を刻みの倍数で挟み、
/// `div` は 0 を中心に正負で同じ幅をとる（`docs/design/README.md`「濃淡の色」）。
pub fn edges(palette: Palette, lo: f64, hi: f64, steps: usize) -> Vec<f64> {
    let (lo, hi) = (lo.min(hi), hi.max(lo));
    match palette {
        Palette::Seq => {
            let step = nice_step(hi - lo, steps);
            let a = (lo / step + 1e-9).floor() as i64;
            let b = (hi / step - 1e-9).ceil() as i64;
            let b = b.max(a + 1);
            (a..=b).map(|k| k as f64 * step).collect()
        }
        Palette::Div => {
            let m = lo.abs().max(hi.abs());
            let step = nice_step(2.0 * m, steps);
            let k = ((m / step - 1e-9).ceil() as i64).max(1);
            (-k..=k).map(|i| i as f64 * step).collect()
        }
    }
}

/// 帯の境の刻み（隣り合う境の差）。
pub fn step_of(edges: &[f64]) -> f64 {
    match edges {
        [a, b, ..] => b - a,
        _ => 1.0,
    }
}

/// 水準の数値を、末位を刻みの位にして書く（刻み 0.5 なら小数 1 桁、2.5 なら 2.5 の 5 の位まで）。
/// 書き方は表の数と同じ `numerus`（3 桁区切り、0.001 ≦ |x| < 1,000,000 の外は E）。0 に負号は付けない。
pub fn format_level(value: f64, step: f64) -> String {
    let decade = step.log10().floor();
    let mantissa = step / 10f64.powf(decade);
    let mut place = decade as i32;
    if (mantissa - 2.5).abs() < 1e-9 {
        place -= 1;
    }
    numerus::at_place(value, place)
}

/// 継ぎ目: 範囲と目安 → 境。期待値は 1・2・2.5・5 × 10ⁿ の規則から手で出した。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_is_rounded_to_1_2_2_5_5() {
        assert_eq!(nice_step(7.0, 10), 1.0);
        assert_eq!(nice_step(30.0, 16), 2.0);
        assert_eq!(nice_step(40.0, 16), 2.5);
        assert_eq!(nice_step(1.0, 10), 0.1);
        assert_eq!(nice_step(3.0, 10), 0.5);
        assert_eq!(nice_step(0.0042, 10), 0.0005);
        assert_eq!(nice_step(0.0, 10), 0.1);
    }

    #[test]
    fn seq_edges_bracket_the_range() {
        assert_eq!(
            edges(Palette::Seq, 0.0, 7.3, 10),
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
        );
        let e = edges(Palette::Seq, 0.12, 2.9, 10);
        assert_eq!(e, vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0]);
        let e = edges(Palette::Seq, -0.3, 0.7, 10);
        assert_eq!(e.len(), 11);
        assert!(
            (e[0] + 0.3).abs() < 1e-12 && (e[10] - 0.7).abs() < 1e-12,
            "{e:?}"
        );
        // 平らな場でも帯が 1 本はある
        let e = edges(Palette::Seq, 2.0, 2.0, 10);
        assert_eq!(e.len(), 2);
        assert!((e[0] - 2.0).abs() < 1e-12 && (e[1] - 2.1).abs() < 1e-12);
    }

    #[test]
    fn div_edges_are_symmetric_about_zero() {
        let e = edges(Palette::Div, -12.3, 4.1, 16);
        assert_eq!(e.len(), 15);
        assert_eq!((e[0], e[7], e[14]), (-14.0, 0.0, 14.0));
        assert!(e.windows(2).all(|w| (w[1] - w[0] - 2.0).abs() < 1e-12));
        assert_eq!(edges(Palette::Div, 0.0, 0.0, 16), vec![-0.1, 0.0, 0.1]);
    }

    #[test]
    fn level_is_written_with_the_digits_of_the_step() {
        assert_eq!(format_level(3.0, 1.0), "3");
        assert_eq!(format_level(-3.0, 1.0), "-3");
        assert_eq!(format_level(2.5, 2.5), "2.5");
        assert_eq!(format_level(0.30000000000000004, 0.1), "0.3");
        assert_eq!(format_level(-0.0, 0.5), "0.0");
        assert_eq!(format_level(1200.0, 200.0), "1,200");
        assert_eq!(format_level(1_200_000.0, 200_000.0), "1.2E+06");
        assert_eq!(format_level(-0.0005, 0.0005), "-5E-04");
        assert_eq!(format_level(0.00025, 0.00025), "2.5E-04");
        assert_eq!(format_level(0.025, 0.025), "0.025");
        assert_eq!(step_of(&[-2.0, 0.0, 2.0]), 2.0);
    }
}
