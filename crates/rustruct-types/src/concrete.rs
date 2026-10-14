//! コンクリート。言葉は `docs/materials/GLOSSARY.md`、値と判断は `docs/materials/adr/0001`。
//! 持つのは Fc と種類だけで、単位体積重量とヤング係数はそこから導く。

use std::fmt;

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
    /// 気乾単位体積重量の表（慣例の値、`docs/materials/adr/0001`）。区分を下から並べ、最後の上限が表の上限。
    /// 表は「無筋コンクリートの単位容積重量」の列。[`Concrete::unit_weight`] もページの表もここから引く
    pub fn unit_weight_table(self) -> &'static [UnitWeightRange] {
        const fn row(fc_upper: f64, unit_weight: f64) -> UnitWeightRange {
            UnitWeightRange {
                fc_upper: Stress::n_per_mm2(fc_upper),
                unit_weight: UnitWeight::kn_per_m3(unit_weight),
            }
        }
        match self {
            Self::Normal => {
                &const {
                    [
                        row(36.0, 23.0),
                        row(48.0, 23.5),
                        row(60.0, 24.0),
                        row(80.0, 24.5),
                        row(FC_MAX, 25.0),
                    ]
                }
            }
        }
    }
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

    /// 気乾単位体積重量 γ。Fc が上限以下になる最初の区分の値。表の上限を超えても最後の区分の値を使う
    /// （ワーニングは warning が言う）
    pub fn unit_weight(self) -> UnitWeight {
        let table = self.kind.unit_weight_table();
        let row = table
            .iter()
            .find(|row| self.fc <= row.fc_upper)
            .or(table.last())
            .expect("表の区分は1つ以上");
        row.unit_weight
    }

    /// 鉄筋コンクリートの単位体積重量 γ₁ = γ + 1 kN/m³
    pub fn rc_unit_weight(self) -> UnitWeight {
        UnitWeight::kn_per_m3(self.unit_weight().as_kn_per_m3() + 1.0)
    }

    /// ヤング係数 E = 3.35×10⁴ × (γ/24)² × (Fc/60)^(1/3) [N/mm²]。γ は kN/m³、Fc は N/mm²。
    /// RC 規準（2018 年版）の式
    pub fn young_modulus(self) -> Stress {
        let gamma = self.unit_weight().as_kn_per_m3();
        let fc = self.fc.as_n_per_mm2();
        Stress::n_per_mm2(3.35e4 * (gamma / 24.0).powi(2) * (fc / 60.0).cbrt())
    }
}
