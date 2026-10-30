//! f64 → 文字列。数を「末位の位（10^place）で丸めた整数の数字の並び」にしてから、小数か E で書く。

/// 小数で書く数の最上位の位（0.001 ≦ |x| < 1,000,000）。丸めた後の数の最上位がこの外なら E で書く
const DECIMAL_EXPONENTS: std::ops::Range<i32> = -3..6;

/// 有効数字 `figures` 桁で書く。末尾の 0 も桁として残す（0.560）。整数部が桁を超えるときは整数で（1,140）。
/// 0 は「0」。規格の丸め（JIS の丸めなど）は呼ぶ側で済ませ、ここは表示の桁をそろえて書くだけにする
pub fn significant(x: f64, figures: u32) -> String {
    if x == 0.0 || !x.is_finite() {
        return at_place(x, 0);
    }
    let figures = figures.max(1) as i32;
    let place = x.abs().log10().floor() as i32 - figures + 1;
    let units = units(x.abs(), place);
    if units >= 10u128.pow(figures as u32) {
        // 9.996 → 10.0 のように丸めで桁が上がった。末位を 1 つ上げて桁の数を保つ
        return write(x.is_sign_negative(), units / 10, place + 1);
    }
    write(x.is_sign_negative(), units, place)
}

/// 小数 `decimals` 桁で書く（`format!("{:.1}")` と同じ桁）。丸めて 0 になれば「0.0」のように 0 を並べる
pub fn fixed(x: f64, decimals: u32) -> String {
    at_place(x, -(decimals as i32))
}

/// 末位を 10^`place` の位にして書く（`place` = -2 なら小数 2 桁、2 なら 100 の位まで）。
/// E で書くときも末位の位は同じで、仮数の桁の数がそれで決まる
pub fn at_place(x: f64, place: i32) -> String {
    if !x.is_finite() {
        return x.to_string();
    }
    write(x.is_sign_negative(), units(x.abs(), place), place)
}

/// |x| を 10^`place` 単位の整数に丸める。2 進の誤差を落としてから四捨五入する
/// （3.465 が 3.4649999… になって 3.46 に落ちないように）
fn units(abs: f64, place: i32) -> u128 {
    let scaled = abs / 10f64.powi(place);
    ((scaled * 1e6).round() / 1e6).round() as u128
}

/// 数字の並び `units` × 10^`place` を書く。0 に負号は付けない
fn write(negative: bool, units: u128, place: i32) -> String {
    let sign = if negative && units != 0 { "-" } else { "" };
    let digits = units.to_string();
    let exponent = digits.len() as i32 - 1 + place;
    if units != 0 && !DECIMAL_EXPONENTS.contains(&exponent) {
        let (head, tail) = digits.split_at(1);
        let point = if tail.is_empty() { "" } else { "." };
        let exponent_sign = if exponent < 0 { '-' } else { '+' };
        return format!(
            "{sign}{head}{point}{tail}E{exponent_sign}{:02}",
            exponent.abs()
        );
    }
    if place >= 0 {
        let zeros = if units == 0 { 0 } else { place as usize };
        return format!(
            "{sign}{}",
            grouped(&format!("{digits}{}", "0".repeat(zeros)))
        );
    }
    let decimals = (-place) as usize;
    let digits = format!("{digits:0>width$}", width = decimals + 1);
    let (whole, fraction) = digits.split_at(digits.len() - decimals);
    format!("{sign}{}.{fraction}", grouped(whole))
}

/// 整数部を 3 桁ごとに「,」で区切る（1140 → 1,140）
fn grouped(whole: &str) -> String {
    let mut out = String::new();
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_figures_are_written_as_decimals() {
        assert_eq!(significant(1.23456, 3), "1.23");
        assert_eq!(significant(0.56, 3), "0.560");
        assert_eq!(significant(-21.04, 3), "-21.0");
    }

    #[test]
    fn the_whole_part_is_grouped_by_three_up_to_six_digits() {
        assert_eq!(significant(1140.0, 4), "1,140");
        assert_eq!(significant(999_999.0, 6), "999,999");
        assert_eq!(significant(-15_512.0, 3), "-15,500");
        assert_eq!(significant(123_456.0, 3), "123,000");
    }

    #[test]
    fn a_million_and_above_is_written_in_e() {
        assert_eq!(significant(1_000_000.0, 6), "1.00000E+06");
        assert_eq!(significant(1_234_567.0, 3), "1.23E+06");
        assert_eq!(significant(-2.5e12, 2), "-2.5E+12");
    }

    #[test]
    fn a_number_rounding_up_to_a_million_is_written_in_e() {
        assert_eq!(significant(999_999.6, 6), "1.00000E+06");
        assert_eq!(significant(999_999.0, 3), "1.00E+06");
    }

    #[test]
    fn the_fraction_is_not_grouped_down_to_a_thousandth() {
        assert_eq!(significant(0.001, 3), "0.00100");
        assert_eq!(significant(0.00123, 3), "0.00123");
        assert_eq!(significant(1234.5678, 8), "1,234.5678");
    }

    #[test]
    fn below_a_thousandth_is_written_in_e() {
        assert_eq!(significant(0.000999, 3), "9.99E-04");
        assert_eq!(significant(-0.00000152, 3), "-1.52E-06");
        assert_eq!(significant(1.52e-12, 3), "1.52E-12");
        assert_eq!(significant(0.0002, 1), "2E-04");
    }

    #[test]
    fn a_number_rounding_up_to_a_thousandth_is_written_as_a_decimal() {
        assert_eq!(significant(0.00099996, 3), "0.00100");
        assert_eq!(significant(0.0000099996, 3), "1.00E-05");
    }

    #[test]
    fn rounding_up_a_digit_keeps_the_figures() {
        assert_eq!(significant(9.996, 3), "10.0");
        assert_eq!(significant(-99.96, 3), "-100");
    }

    #[test]
    fn halves_lost_to_binary_error_still_round_up() {
        // 3.465 は 2 進で 3.46499999…
        assert_eq!(significant(3.465, 3), "3.47");
        assert_eq!(fixed(2.675, 2), "2.68");
    }

    #[test]
    fn zero_has_no_sign() {
        assert_eq!(significant(0.0, 3), "0");
        assert_eq!(significant(-0.0, 3), "0");
        assert_eq!(fixed(-0.001, 1), "0.0");
        assert_eq!(at_place(-0.0, -1), "0.0");
    }

    #[test]
    fn fixed_decimals_follow_the_same_ranges() {
        assert_eq!(fixed(23.0, 1), "23.0");
        assert_eq!(fixed(22_723.4, 0), "22,723");
        assert_eq!(fixed(0.00052, 4), "5E-04");
        assert_eq!(fixed(1_234_567.0, 0), "1.234567E+06");
    }

    #[test]
    fn a_place_above_the_units_writes_trailing_zeros() {
        assert_eq!(at_place(1200.0, 2), "1,200");
        assert_eq!(at_place(1_200_000.0, 5), "1.2E+06");
        assert_eq!(at_place(0.0025, -4), "0.0025");
    }
}
