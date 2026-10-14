//! 鉄筋（JIS G 3112:2025 の異形棒鋼）。言葉は `docs/materials/GLOSSARY.md`、値と出典は
//! `docs/materials/adr/0002`。呼び名が持つのは公称直径だけで、ほかの諸元は JIS と同じ式と丸めで求める。

use crate::jis_rounding::Decimal;
use crate::quantity::{Area, Length, LinearMass};

/// 鉄筋の種類。JIS G 3112 表1 の異形棒鋼の記号。強度は使う Tool の一本で規準と一緒に決める。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RebarGrade {
    Sd295,
    Sd345,
    Sd390,
    Sd490,
    Sd590A,
    Sd590B,
    Sd685A,
    Sd685B,
    /// 主にせん断補強筋に用いる（表1 注 a）
    Sd685R,
    /// 主にせん断補強筋に用いる（表1 注 a）
    Sd785R,
}

impl RebarGrade {
    pub const ALL: [Self; 10] = [
        Self::Sd295,
        Self::Sd345,
        Self::Sd390,
        Self::Sd490,
        Self::Sd590A,
        Self::Sd590B,
        Self::Sd685A,
        Self::Sd685B,
        Self::Sd685R,
        Self::Sd785R,
    ];

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Sd295 => "SD295",
            Self::Sd345 => "SD345",
            Self::Sd390 => "SD390",
            Self::Sd490 => "SD490",
            Self::Sd590A => "SD590A",
            Self::Sd590B => "SD590B",
            Self::Sd685A => "SD685A",
            Self::Sd685B => "SD685B",
            Self::Sd685R => "SD685R",
            Self::Sd785R => "SD785R",
        }
    }
}

/// 呼び名。JIS G 3112 表6 の異形棒鋼。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarSize {
    D4,
    D5,
    D6,
    D8,
    D10,
    D13,
    D16,
    D19,
    D22,
    D25,
    D29,
    D32,
    D35,
    D38,
    D41,
    D51,
}

/// 公称周長 l = 3.142 × d（表6 注 a）
const PERIMETER_COEFFICIENT: Decimal = Decimal::new(3142, 3);
/// 公称断面積 S = 0.7854 × d²（表6 注 a）
const AREA_COEFFICIENT: Decimal = Decimal::new(7854, 4);
/// 単位質量 w = 7.85 × 10⁻³ × S。鋼 1 cm³ を 7.85 g とする（表6 注 a）。S [mm²] から w [kg/m]
const MASS_COEFFICIENT: Decimal = Decimal::new(785, 5);

impl BarSize {
    pub const ALL: [Self; 16] = [
        Self::D4,
        Self::D5,
        Self::D6,
        Self::D8,
        Self::D10,
        Self::D13,
        Self::D16,
        Self::D19,
        Self::D22,
        Self::D25,
        Self::D29,
        Self::D32,
        Self::D35,
        Self::D38,
        Self::D41,
        Self::D51,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::D4 => "D4",
            Self::D5 => "D5",
            Self::D6 => "D6",
            Self::D8 => "D8",
            Self::D10 => "D10",
            Self::D13 => "D13",
            Self::D16 => "D16",
            Self::D19 => "D19",
            Self::D22 => "D22",
            Self::D25 => "D25",
            Self::D29 => "D29",
            Self::D32 => "D32",
            Self::D35 => "D35",
            Self::D38 => "D38",
            Self::D41 => "D41",
            Self::D51 => "D51",
        }
    }

    /// 公称直径 d [mm]。表6 の値。呼び名が持つ唯一の値
    fn diameter(self) -> Decimal {
        let hundredths_of_mm = match self {
            Self::D4 => 423,
            Self::D5 => 529,
            Self::D6 => 635,
            Self::D8 => 794,
            Self::D10 => 953,
            Self::D13 => 1270,
            Self::D16 => 1590,
            Self::D19 => 1910,
            Self::D22 => 2220,
            Self::D25 => 2540,
            Self::D29 => 2860,
            Self::D32 => 3180,
            Self::D35 => 3490,
            Self::D38 => 3810,
            Self::D41 => 4130,
            Self::D51 => 5080,
        };
        Decimal::new(hundredths_of_mm, 2)
    }

    /// 公称断面積 S [mm²]。有効数字 4 桁に規則A で丸める。単位質量はこの丸めた値から求める
    fn area(self) -> Decimal {
        let d = self.diameter();
        AREA_COEFFICIENT.mul(d).mul(d).round_significant(4)
    }

    pub fn nominal_diameter(self) -> Length {
        Length::mm(self.diameter().to_f64())
    }

    /// 公称周長 l。小数点以下 1 桁に規則A で丸める
    pub fn nominal_perimeter(self) -> Length {
        let l = PERIMETER_COEFFICIENT
            .mul(self.diameter())
            .round_decimal_places(1);
        Length::mm(l.to_f64())
    }

    pub fn nominal_area(self) -> Area {
        Area::mm2(self.area().to_f64())
    }

    /// 単位質量 w。丸めた公称断面積から求め、有効数字 3 桁に規則A で丸める
    pub fn unit_mass(self) -> LinearMass {
        let w = MASS_COEFFICIENT.mul(self.area()).round_significant(3);
        LinearMass::kg_per_m(w.to_f64())
    }
}
