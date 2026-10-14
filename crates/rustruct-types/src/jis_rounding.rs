//! JIS Z 8401 の規則A（偶数丸め）。2進の浮動小数点では 10 進の丸めの中間を正しく判定できないので、
//! 値は 10 進の整数（仮数 × 10^-桁）で持ち、丸めも整数で行う。`docs/materials/adr/0002`。

/// 10 進で正確な数。`mantissa × 10^-scale`。末尾の 0 は常に落とした形で持つので、等しい数は等しい値になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Decimal {
    mantissa: i128,
    scale: i32,
}

impl Decimal {
    pub(crate) const fn new(mantissa: i128, scale: i32) -> Self {
        let (mut mantissa, mut scale) = (mantissa, scale);
        while mantissa != 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        if mantissa == 0 {
            scale = 0;
        }
        Self { mantissa, scale }
    }

    pub(crate) const fn mul(self, other: Self) -> Self {
        Self::new(self.mantissa * other.mantissa, self.scale + other.scale)
    }

    /// 小数点以下 `places` 桁に丸める（規則A）。
    pub(crate) fn round_decimal_places(self, places: i32) -> Self {
        self.drop_digits(self.scale - places)
    }

    /// 有効数字 `figures` 桁に丸める（規則A）。
    pub(crate) fn round_significant(self, figures: u32) -> Self {
        let digits = self
            .mantissa
            .unsigned_abs()
            .checked_ilog10()
            .map_or(0, |d| d + 1);
        self.drop_digits(digits as i32 - figures as i32)
    }

    /// 仮数の下 `count` 桁を落とし、規則A で丸める: 丸めの幅の整数倍のうち近い方、等しく近ければ偶数倍の方。
    fn drop_digits(self, count: i32) -> Self {
        if count <= 0 {
            return self;
        }
        let width = 10_i128.pow(count as u32);
        let magnitude = self.mantissa.abs();
        let (mut quotient, remainder) = (magnitude / width, magnitude % width);
        if 2 * remainder > width || (2 * remainder == width && quotient % 2 == 1) {
            quotient += 1;
        }
        Self::new(self.mantissa.signum() * quotient, self.scale - count)
    }

    pub(crate) fn to_f64(self) -> f64 {
        // 仮数と 10 の冪がどちらも f64 で正確なうちは、割り算1回で最も近い f64 になる
        if self.scale >= 0 {
            self.mantissa as f64 / 10_f64.powi(self.scale)
        } else {
            self.mantissa as f64 * 10_f64.powi(-self.scale)
        }
    }

    #[cfg(test)]
    fn parse(s: &str) -> Self {
        let (int, frac) = s.split_once('.').unwrap_or((s, ""));
        let mantissa = format!("{int}{frac}").parse().expect("10 進の数");
        Self::new(mantissa, frac.len() as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Decimal {
        Decimal::parse(s)
    }

    #[test]
    fn rule_a_rounds_a_tie_to_the_even_neighbour() {
        // JIS Z 8401 の例: 丸めの幅 0.1 で 12.25 → 12.2、12.35 → 12.4
        assert_eq!(dec("12.25").round_decimal_places(1), dec("12.2"));
        assert_eq!(dec("12.35").round_decimal_places(1), dec("12.4"));
    }

    #[test]
    fn rule_a_rounds_a_non_tie_to_the_nearest() {
        assert_eq!(dec("12.251").round_decimal_places(1), dec("12.3"));
        assert_eq!(dec("12.249").round_decimal_places(1), dec("12.2"));
    }

    #[test]
    fn significant_figures_count_from_the_first_non_zero_digit() {
        assert_eq!(dec("126.6769").round_significant(4), dec("126.7"));
        assert_eq!(dec("0.0123450").round_significant(4), dec("0.01234"));
        assert_eq!(dec("1139.5").round_significant(4), dec("1140"));
        assert_eq!(dec("1138.5").round_significant(4), dec("1138"));
    }

    #[test]
    fn products_are_exact() {
        assert_eq!(
            dec("0.7854").mul(dec("9.53")).mul(dec("9.53")),
            dec("71.33073486")
        );
    }
}
