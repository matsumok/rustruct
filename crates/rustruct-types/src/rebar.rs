//! 鉄筋（JIS G 3112:2025 の異形棒鋼）。言葉は `docs/materials/GLOSSARY.md`、値と出典は
//! `docs/materials/adr/0002`。呼び名が持つのは公称直径だけで、ほかの諸元は JIS と同じ式と丸めで求める。
//! 種類と呼び名は、列挙子・一覧・記号・値を一つの表から作り、一覧への足し忘れを起こさない。

use crate::jis_rounding::Decimal;
use crate::quantity::{Area, Length, LinearMass};

macro_rules! rebar_kinds {
    ($($(#[$doc:meta])* $variant:ident => $symbol:literal,)+) => {
        /// 鉄筋の種類。JIS G 3112 表1 の異形棒鋼の記号。強度は使う Tool の一本で規準と一緒に決める。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum RebarKind {
            $($(#[$doc])* $variant,)+
        }

        impl RebarKind {
            pub const ALL: &[Self] = &[$(Self::$variant,)+];

            pub fn symbol(self) -> &'static str {
                match self {
                    $(Self::$variant => $symbol,)+
                }
            }
        }
    };
}

rebar_kinds! {
    Sd295 => "SD295",
    Sd345 => "SD345",
    Sd390 => "SD390",
    Sd490 => "SD490",
    Sd590A => "SD590A",
    Sd590B => "SD590B",
    Sd685A => "SD685A",
    Sd685B => "SD685B",
    /// 主にせん断補強筋に用いる（表1 注 a）
    Sd685R => "SD685R",
    /// 主にせん断補強筋に用いる（表1 注 a）
    Sd785R => "SD785R",
}

macro_rules! bar_designations {
    ($($variant:ident => $hundredths_of_mm:literal,)+) => {
        /// 呼び名。JIS G 3112 表6 の異形棒鋼。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum BarDesignation {
            $($variant,)+
        }

        impl BarDesignation {
            pub const ALL: &[Self] = &[$(Self::$variant,)+];

            pub fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant),)+
                }
            }

            /// 公称直径 d [mm] を 10 進で正確に。表6 の値。呼び名が持つ唯一の値
            fn exact_diameter(self) -> Decimal {
                match self {
                    $(Self::$variant => Decimal::new($hundredths_of_mm, 2),)+
                }
            }
        }
    };
}

// 呼び名 => 公称直径 [0.01 mm]
bar_designations! {
    D4 => 423,
    D5 => 529,
    D6 => 635,
    D8 => 794,
    D10 => 953,
    D13 => 1270,
    D16 => 1590,
    D19 => 1910,
    D22 => 2220,
    D25 => 2540,
    D29 => 2860,
    D32 => 3180,
    D35 => 3490,
    D38 => 3810,
    D41 => 4130,
    D51 => 5080,
}

/// 公称周長 l = 3.142 × d（表6 注 a）
const PERIMETER_COEFFICIENT: Decimal = Decimal::new(3142, 3);
/// 公称断面積 S = 0.7854 × d²（表6 注 a）
const AREA_COEFFICIENT: Decimal = Decimal::new(7854, 4);
/// 単位質量 w = 7.85 × 10⁻³ × S。鋼 1 cm³ を 7.85 g とする（表6 注 a）。S [mm²] から w [kg/m]
const MASS_COEFFICIENT: Decimal = Decimal::new(785, 5);

impl BarDesignation {
    /// 公称断面積 S [mm²] を 10 進で正確に。有効数字 4 桁に規則A で丸めた値。単位質量はこの値から求める
    fn exact_area(self) -> Decimal {
        let d = self.exact_diameter();
        AREA_COEFFICIENT.mul(d).mul(d).round_significant(4)
    }

    pub fn nominal_diameter(self) -> Length {
        Length::mm(self.exact_diameter().to_f64())
    }

    /// 公称周長 l。小数点以下 1 桁に規則A で丸める
    pub fn nominal_perimeter(self) -> Length {
        let l = PERIMETER_COEFFICIENT
            .mul(self.exact_diameter())
            .round_decimal_places(1);
        Length::mm(l.to_f64())
    }

    pub fn nominal_area(self) -> Area {
        Area::mm2(self.exact_area().to_f64())
    }

    /// 単位質量 w。丸めた公称断面積から求め、有効数字 3 桁に規則A で丸める
    pub fn unit_mass(self) -> LinearMass {
        let w = MASS_COEFFICIENT.mul(self.exact_area()).round_significant(3);
        LinearMass::kg_per_m(w.to_f64())
    }
}
