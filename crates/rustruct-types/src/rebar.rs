//! 鉄筋（JIS G 3112:2025 の異形棒鋼）。言葉は `docs/materials/GLOSSARY.md`、値と出典は
//! `docs/materials/adr/0002`（諸元）と `docs/materials/adr/0003`（許容応力度）。呼び名が持つのは公称直径だけで、ほかの諸元は JIS と同じ式と丸めで求める。
//! 種類と呼び名は、列挙子・一覧・記号・値を一つの表から作り、一覧への足し忘れを起こさない。

use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;

use crate::allowable_stress::AllowableStress;
use crate::quantity::{Area, Length, LinearMass, Stress};

/// 鉄筋の種類の表の1行。列が増えたらフィールドを足す。許容応力度は RC 規準 表 6.2 [N/mm²]
struct KindRow {
    symbol: &'static str,
    /// 長期 引張・圧縮（D25 以下）
    long: f64,
    /// 長期 引張・圧縮（D29 以上。表 6.2 注 *）
    long_d29_or_larger: f64,
    /// 長期 せん断補強
    long_shear: f64,
    /// 短期 引張・圧縮
    short: f64,
    /// 短期 せん断補強
    short_shear: f64,
}

macro_rules! rebar_kinds {
    ($($(#[$doc:meta])* $variant:ident => $symbol:literal {
        long: $long:literal,
        long_d29_or_larger: $long_d29:literal,
        long_shear: $long_shear:literal,
        short: $short:literal,
        short_shear: $short_shear:literal $(,)?
    },)+) => {
        /// 鉄筋の種類。JIS G 3112 表1 の異形棒鋼の記号のうち、RC 規準 表 6.2 に許容応力度があるもの。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum RebarKind {
            $($(#[$doc])* $variant,)+
        }

        impl RebarKind {
            pub const ALL: &[Self] = &[$(Self::$variant,)+];

            fn row(self) -> &'static KindRow {
                match self {
                    $(Self::$variant => &KindRow {
                        symbol: $symbol,
                        long: $long,
                        long_d29_or_larger: $long_d29,
                        long_shear: $long_shear,
                        short: $short,
                        short_shear: $short_shear,
                    },)+
                }
            }
        }
    };
}

// 表 6.2 の「SD295A および B」は JIS G 3112:2020 で SD295 に一本化された
rebar_kinds! {
    Sd295 => "SD295" { long: 195.0, long_d29_or_larger: 195.0, long_shear: 195.0, short: 295.0, short_shear: 295.0 },
    Sd345 => "SD345" { long: 215.0, long_d29_or_larger: 195.0, long_shear: 195.0, short: 345.0, short_shear: 345.0 },
    Sd390 => "SD390" { long: 215.0, long_d29_or_larger: 195.0, long_shear: 195.0, short: 390.0, short_shear: 390.0 },
    Sd490 => "SD490" { long: 215.0, long_d29_or_larger: 195.0, long_shear: 195.0, short: 490.0, short_shear: 490.0 },
    // RC 規準 表 6.2 に許容応力度が無いので扱わない（2026-10-14 本人の判断）。値の出どころを決めたら戻す
    // Sd590A => "SD590A",
    // Sd590B => "SD590B",
    // Sd685A => "SD685A",
    // Sd685B => "SD685B",
    // /// 主にせん断補強筋に用いる（JIS G 3112 表1 注 a）
    // Sd685R => "SD685R",
    // /// 主にせん断補強筋に用いる（JIS G 3112 表1 注 a）
    // Sd785R => "SD785R",
}

impl RebarKind {
    pub fn symbol(self) -> &'static str {
        self.row().symbol
    }

    /// 引張・圧縮の許容応力度（RC 規準 表 6.2）。長期は D29 以上の太さで小さくなる（注 *）
    pub fn allowable_tension_compression(self, designation: BarDesignation) -> AllowableStress {
        let row = self.row();
        let long = if designation.nominal_diameter() >= BarDesignation::D29.nominal_diameter() {
            row.long_d29_or_larger
        } else {
            row.long
        };
        AllowableStress {
            long_term: Stress::n_per_mm2(long),
            short_term: Stress::n_per_mm2(row.short),
        }
    }

    /// せん断補強筋としての許容引張応力度（RC 規準 表 6.2）
    pub fn allowable_shear_reinforcement(self) -> AllowableStress {
        let row = self.row();
        AllowableStress {
            long_term: Stress::n_per_mm2(row.long_shear),
            short_term: Stress::n_per_mm2(row.short_shear),
        }
    }
}

/// 呼び名の表の1行。列が増えたらフィールドを足す
struct DesignationRow {
    name: &'static str,
    /// 公称直径 d [mm]。表6 の値。呼び名が持つ唯一の値
    diameter_mm: Decimal,
}

macro_rules! bar_designations {
    ($($variant:ident => $diameter_mm:literal,)+) => {
        /// 呼び名。JIS G 3112 表6 の異形棒鋼。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum BarDesignation {
            $($variant,)+
        }

        impl BarDesignation {
            pub const ALL: &[Self] = &[$(Self::$variant,)+];

            fn row(self) -> DesignationRow {
                match self {
                    $(Self::$variant => DesignationRow {
                        name: stringify!($variant),
                        diameter_mm: dec!($diameter_mm),
                    },)+
                }
            }
        }
    };
}

// 呼び名 => 公称直径 [mm]
bar_designations! {
    D4 => 4.23,
    D5 => 5.29,
    D6 => 6.35,
    D8 => 7.94,
    D10 => 9.53,
    D13 => 12.7,
    D16 => 15.9,
    D19 => 19.1,
    D22 => 22.2,
    D25 => 25.4,
    D29 => 28.6,
    D32 => 31.8,
    D35 => 34.9,
    D38 => 38.1,
    D41 => 41.3,
    D51 => 50.8,
}

/// 公称周長 l = 3.142 × d（表6 注 a）
const PERIMETER_COEFFICIENT: Decimal = dec!(3.142);
/// 公称断面積 S = 0.7854 × d²（表6 注 a）
const AREA_COEFFICIENT: Decimal = dec!(0.7854);
/// 単位質量 w = 7.85 × 10⁻³ × S。鋼 1 cm³ を 7.85 g とする（表6 注 a）。S [mm²] から w [kg/m]
const MASS_COEFFICIENT: Decimal = dec!(0.00785);
/// JIS Z 8401 の規則A: 丸めの幅の整数倍のうち近い方、等しく近ければ偶数倍の方
const RULE_A: RoundingStrategy = RoundingStrategy::MidpointNearestEven;

impl BarDesignation {
    pub fn name(self) -> &'static str {
        self.row().name
    }

    /// 公称直径 d [mm] を 10 進で正確に
    fn exact_diameter(self) -> Decimal {
        self.row().diameter_mm
    }

    /// 公称断面積 S [mm²] を 10 進で正確に。有効数字 4 桁に規則A で丸めた値。単位質量はこの値から求める
    fn exact_area(self) -> Decimal {
        let d = self.exact_diameter();
        (AREA_COEFFICIENT * d * d)
            .round_sf_with_strategy(4, RULE_A)
            .expect("桁は 28 桁に遠く及ばない")
    }

    pub fn nominal_diameter(self) -> Length {
        Length::mm(to_f64(self.exact_diameter()))
    }

    /// 公称周長 l。小数点以下 1 桁に規則A で丸める
    pub fn nominal_perimeter(self) -> Length {
        let l = (PERIMETER_COEFFICIENT * self.exact_diameter()).round_dp_with_strategy(1, RULE_A);
        Length::mm(to_f64(l))
    }

    pub fn nominal_area(self) -> Area {
        Area::mm2(to_f64(self.exact_area()))
    }

    /// 単位質量 w。丸めた公称断面積から求め、有効数字 3 桁に規則A で丸める
    pub fn unit_mass(self) -> LinearMass {
        let w = (MASS_COEFFICIENT * self.exact_area())
            .round_sf_with_strategy(3, RULE_A)
            .expect("桁は 28 桁に遠く及ばない");
        LinearMass::kg_per_m(to_f64(w))
    }
}

fn to_f64(value: Decimal) -> f64 {
    value.to_f64().expect("諸元は f64 に収まる")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 表6 には丸めのちょうど中間が無く、表との照合では規則A と四捨五入を区別できないので、ここで確かめる
    #[test]
    fn rule_a_rounds_a_tie_to_the_even_neighbour() {
        // JIS Z 8401 の例: 丸めの幅 0.1 で 12.25 → 12.2、12.35 → 12.4
        assert_eq!(dec!(12.25).round_dp_with_strategy(1, RULE_A), dec!(12.2));
        assert_eq!(dec!(12.35).round_dp_with_strategy(1, RULE_A), dec!(12.4));
        assert_eq!(
            dec!(1138.5).round_sf_with_strategy(4, RULE_A),
            Some(dec!(1138))
        );
    }
}
