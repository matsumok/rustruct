//! コンクリート。言葉は `docs/materials/GLOSSARY.md`、値と判断は `docs/materials/adr/0001`。
//! 持つのは Fc と種類だけで、単位体積重量とヤング係数はそこから導く。

use std::fmt;

use crate::allowable_stress::AllowableStress;
use crate::quantity::{Stress, UnitWeight};

/// コンクリートの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConcreteKind {
    /// 普通コンクリート
    Normal,
}

/// 気乾単位体積重量の表の1行。Fc がこの区分の上限以下なら γ はこの値
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitWeightRange {
    /// 区分の上限 Fc
    pub fc_upper: Stress,
    /// 気乾単位体積重量 γ
    pub unit_weight: UnitWeight,
}

impl ConcreteKind {
    /// 気乾単位体積重量の表。区分を下から並べ、最後の上限が表の上限。ページに表を見せるために置く
    pub fn unit_weight_table(self) -> &'static [UnitWeightRange] {
        match self {
            Self::Normal => NORMAL_UNIT_WEIGHT_TABLE,
        }
    }
}

/// 「Fc の上限 => γ」の1つの表から、γ を引く match と、ページに見せる表の両方を作る。
/// 表の上限を超えた Fc は最後の区分の値を使う（ワーニングは `Concrete::warning` が言う）
macro_rules! unit_weight_table {
    ($table:ident, $lookup:ident, $(..= $fc_upper:tt => $kn_per_m3:literal,)+) => {
        const $table: &[UnitWeightRange] = &[$(UnitWeightRange {
            fc_upper: Stress::n_per_mm2($fc_upper),
            unit_weight: UnitWeight::kn_per_m3($kn_per_m3),
        },)+];

        fn $lookup(fc: Stress) -> UnitWeight {
            match fc.as_n_per_mm2() {
                $(..= $fc_upper => UnitWeight::kn_per_m3($kn_per_m3),)+
                _ => $table[$table.len() - 1].unit_weight,
            }
        }
    };
}

// 普通コンクリートの気乾単位体積重量（慣例の値、`docs/materials/adr/0001`）。
// Fc の区分の上限 [N/mm²] => γ [kN/m³]。表は「無筋コンクリートの単位容積重量」の列
unit_weight_table! {
    NORMAL_UNIT_WEIGHT_TABLE, normal_unit_weight,
    ..=36.0 => 23.0,
    ..=48.0 => 23.5,
    ..=60.0 => 24.0,
    ..=80.0 => 24.5,
    ..=FC_MAX => 25.0,
}

/// 鉄筋の位置。許容付着応力度を分ける（表 6.3 注 1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarPosition {
    /// 上端筋: 曲げ材で、その鉄筋の下に 300 mm 以上のコンクリートが打ち込まれる水平鉄筋
    Top,
    /// その他の鉄筋
    Other,
}

/// 設計基準強度と種類で特定されるコンクリート。Fc は正の有限値に限る。
/// 18 以上 100 以下（N/mm²）を外れても作れて、計算もできる。外れていることは [`Concrete::warning`] が言う。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Concrete {
    fc: Stress,
    kind: ConcreteKind,
}

/// コンクリートを作れない理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcreteError {
    /// Fc が正の有限値でない
    NotPositive,
}

impl fmt::Display for ConcreteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPositive => f.write_str("Fc が正の有限値でない"),
        }
    }
}

impl std::error::Error for ConcreteError {}

/// 作れて計算もできるが、Fc が想定の範囲を外れていること。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcreteWarning {
    /// Fc が下限（18 N/mm²）に満たない
    BelowMinimum,
    /// Fc が単位体積重量の表の上限（100 N/mm²）を超える。γ は最後の区分の値を使う
    AboveTable,
}

impl fmt::Display for ConcreteWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BelowMinimum => write!(f, "Fc が下限 {FC_MIN} N/mm² に満たない"),
            Self::AboveTable => write!(
                f,
                "Fc が単位体積重量の表の上限 {FC_MAX} N/mm² を超える。γ は最後の区分の値を使う"
            ),
        }
    }
}

/// Fc の下限 [N/mm²]。これに満たないとワーニング
const FC_MIN: f64 = 18.0;

/// 単位体積重量の表の上限 [N/mm²]。これを超えるとワーニング
const FC_MAX: f64 = 100.0;

impl Concrete {
    pub fn new(fc: Stress, kind: ConcreteKind) -> Result<Self, ConcreteError> {
        let value = fc.as_n_per_mm2();
        if !value.is_finite() || value <= 0.0 {
            return Err(ConcreteError::NotPositive);
        }
        Ok(Self { fc, kind })
    }

    /// Fc が想定の範囲（18 以上 100 以下）を外れていれば、そのワーニング
    pub fn warning(self) -> Option<ConcreteWarning> {
        let fc = self.fc.as_n_per_mm2();
        if fc < FC_MIN {
            Some(ConcreteWarning::BelowMinimum)
        } else if fc > FC_MAX {
            Some(ConcreteWarning::AboveTable)
        } else {
            None
        }
    }

    /// 設計基準強度 Fc
    pub fn fc(self) -> Stress {
        self.fc
    }

    pub fn kind(self) -> ConcreteKind {
        self.kind
    }

    /// 気乾単位体積重量 γ。Fc の区分の表で引く
    pub fn unit_weight(self) -> UnitWeight {
        match self.kind {
            ConcreteKind::Normal => normal_unit_weight(self.fc),
        }
    }

    /// 許容圧縮応力度。長期 Fc/3、短期は長期の 2 倍（RC 規準 表 6.1）
    pub fn allowable_compression(self) -> AllowableStress {
        AllowableStress::short_term_times(Stress::n_per_mm2(self.fc_value() / 3.0), 2.0)
    }

    /// 許容せん断応力度。長期 Fc/30 かつ (0.49 + Fc/100) 以下、短期は長期の 1.5 倍（RC 規準 表 6.1）
    pub fn allowable_shear(self) -> AllowableStress {
        let fc = self.fc_value();
        let long = match self.kind {
            ConcreteKind::Normal => (fc / 30.0).min(0.49 + fc / 100.0),
        };
        AllowableStress::short_term_times(Stress::n_per_mm2(long), 1.5)
    }

    /// 異形鉄筋の許容付着応力度。短期は長期の 1.5 倍（RC 規準 表 6.3）。
    /// かぶりが鉄筋径の 1.5 倍未満のときの低減（注 3）は含まない
    pub fn allowable_bond(self, position: BarPosition) -> AllowableStress {
        let fc = self.fc_value();
        let long = match position {
            // Fc/15 かつ (0.9 + 2Fc/75) 以下
            BarPosition::Top => (fc / 15.0).min(0.9 + 2.0 * fc / 75.0),
            // Fc/10 かつ (1.35 + Fc/25) 以下
            BarPosition::Other => (fc / 10.0).min(1.35 + fc / 25.0),
        };
        AllowableStress::short_term_times(Stress::n_per_mm2(long), 1.5)
    }

    /// Fc [N/mm²]。RC 規準の式は N/mm² で書かれている
    fn fc_value(self) -> f64 {
        self.fc.as_n_per_mm2()
    }

    /// 鉄筋コンクリートの単位体積重量 γ₁ = γ + 1 kN/m³
    pub fn rc_unit_weight(self) -> UnitWeight {
        UnitWeight::kn_per_m3(self.unit_weight().as_kn_per_m3() + 1.0)
    }

    /// ヤング係数 E = 3.35×10⁴ × (γ/24)² × (Fc/60)^(1/3) [N/mm²]。γ は kN/m³、Fc は N/mm²。
    /// RC 規準（2024 年版）の式
    pub fn young_modulus(self) -> Stress {
        let gamma = self.unit_weight().as_kn_per_m3();
        let fc = self.fc.as_n_per_mm2();
        Stress::n_per_mm2(3.35e4 * (gamma / 24.0).powi(2) * (fc / 60.0).cbrt())
    }
}
