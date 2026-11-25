//! 鋼材。言葉は `docs/materials/GLOSSARY.md`、値と判断は ADR materials-0004。
//! 鋼種は、列挙子・一覧・記号・基準強度 F を一つの表から作り、一覧への足し忘れを起こさない。
//! E と ν はどの鋼種でも同じなので、鋼種に依らない関数で読む。

use crate::quantity::{Length, PoissonRatio, Stress};

/// 鋼種の表の1行。列が増えたらフィールドを足す
struct GradeRow {
    symbol: &'static str,
    /// 基準強度 F（t ≤ 40 mm）
    f_up_to_40: Stress,
    /// 基準強度 F（40 < t ≤ 100 mm）。この区分の無い鋼種は `None`
    f_over_40: Option<Stress>,
}

macro_rules! steel_grades {
    ($($variant:ident => $symbol:literal {
        up_to_40: $up_to_40:literal,
        over_40: $over_40:tt $(,)?
    },)+) => {
        /// 鋼種。建物の板・形鋼と角形鋼管でよく使う 12 種（ADR materials-0004）。
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum SteelGrade {
            $($variant,)+
        }

        impl SteelGrade {
            pub const ALL: &[Self] = &[$(Self::$variant,)+];

            fn row(self) -> GradeRow {
                match self {
                    $(Self::$variant => GradeRow {
                        symbol: $symbol,
                        f_up_to_40: Stress::n_per_mm2($up_to_40),
                        f_over_40: steel_grades!(@over_40 $over_40),
                    },)+
                }
            }
        }
    };
    (@over_40 none) => { None };
    (@over_40 $value:literal) => { Some(Stress::n_per_mm2($value)) };
}

// 記号 => 基準強度 F [N/mm²]（t ≤ 40 mm、40 < t ≤ 100 mm）
steel_grades! {
    Ss400 => "SS400" { up_to_40: 235.0, over_40: 215.0 },
    Sn400A => "SN400A" { up_to_40: 235.0, over_40: 215.0 },
    Sn400B => "SN400B" { up_to_40: 235.0, over_40: 215.0 },
    Sn400C => "SN400C" { up_to_40: 235.0, over_40: 215.0 },
    Sn490B => "SN490B" { up_to_40: 325.0, over_40: 295.0 },
    Sn490C => "SN490C" { up_to_40: 325.0, over_40: 295.0 },
    Sm490A => "SM490A" { up_to_40: 325.0, over_40: 295.0 },
    Stkr400 => "STKR400" { up_to_40: 235.0, over_40: none },
    Stkr490 => "STKR490" { up_to_40: 325.0, over_40: none },
    Bcr295 => "BCR295" { up_to_40: 295.0, over_40: none },
    Bcp235 => "BCP235" { up_to_40: 235.0, over_40: 215.0 },
    Bcp325 => "BCP325" { up_to_40: 325.0, over_40: 295.0 },
}

/// 板厚の区分の境。境の値は薄い側の区分に入る
const THICKNESS_BOUNDARY: Length = Length::mm(40.0);
/// 板厚の区分の上限
const THICKNESS_MAX: Length = Length::mm(100.0);

impl SteelGrade {
    pub fn symbol(self) -> &'static str {
        self.row().symbol
    }

    /// 記号（"SN400B" など）から鋼種を引く。大文字と小文字は区別する
    pub fn from_symbol(symbol: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|g| g.symbol() == symbol)
    }

    /// 板厚 `thickness` での基準強度 F。t が正の有限値でないとき、100 mm を超えるとき、
    /// その鋼種に区分の無い厚さのときは `None`
    pub fn standard_strength(self, thickness: Length) -> Option<Stress> {
        let row = self.row();
        // NaN はどの比較でも偽になり、最後の腕に落ちる
        if thickness > Length::mm(0.0) && thickness <= THICKNESS_BOUNDARY {
            Some(row.f_up_to_40)
        } else if thickness > THICKNESS_BOUNDARY && thickness <= THICKNESS_MAX {
            row.f_over_40
        } else {
            None
        }
    }
}

/// 鋼材のヤング係数 E。どの鋼種でも 205000 N/mm²
pub fn young_modulus() -> Stress {
    Stress::n_per_mm2(205000.0)
}

/// 鋼材のポアソン比 ν。どの鋼種でも 0.3
pub fn poisson_ratio() -> PoissonRatio {
    PoissonRatio::new(0.3).expect("0.3 は 0 以上 0.5 未満")
}
