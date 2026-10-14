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

/// 設計基準強度と種類で特定されるコンクリート。Fc は 18 以上 100 以下（N/mm²）に限る。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Concrete {
    fc: Stress,
    kind: ConcreteKind,
}

/// コンクリートを作れない理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcreteError {
    /// Fc が有限値でない
    NotFinite,
    /// Fc が下限（18 N/mm²）に満たない
    BelowMinimum,
    /// Fc が単位体積重量の表の区分（100 N/mm² 以下）を超える
    AboveTable,
}

impl fmt::Display for ConcreteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => f.write_str("Fc が有限値でない"),
            Self::BelowMinimum => write!(f, "Fc が下限 {FC_MIN} N/mm² に満たない"),
            Self::AboveTable => write!(f, "Fc が単位体積重量の表の上限 {FC_MAX} N/mm² を超える"),
        }
    }
}

impl std::error::Error for ConcreteError {}

/// Fc の下限 [N/mm²]。これに満たない Fc のコンクリートは作れない
const FC_MIN: f64 = 18.0;

/// 単位体積重量の表の上限 [N/mm²]。これを超える Fc のコンクリートは作れない
const FC_MAX: f64 = 100.0;

impl Concrete {
    pub fn new(fc: Stress, kind: ConcreteKind) -> Result<Self, ConcreteError> {
        let value = fc.as_n_per_mm2();
        if !value.is_finite() {
            return Err(ConcreteError::NotFinite);
        }
        if value < FC_MIN {
            return Err(ConcreteError::BelowMinimum);
        }
        if value > FC_MAX {
            return Err(ConcreteError::AboveTable);
        }
        Ok(Self { fc, kind })
    }

    /// 設計基準強度 Fc
    pub fn fc(self) -> Stress {
        self.fc
    }

    pub fn kind(self) -> ConcreteKind {
        self.kind
    }

    /// 気乾単位体積重量 γ。Fc の区分の表で引く（慣例の値、`docs/materials/adr/0001`）。
    /// 表は「無筋コンクリートの単位容積重量」の列
    pub fn unit_weight(self) -> UnitWeight {
        let fc = self.fc.as_n_per_mm2();
        let kn_per_m3 = match self.kind {
            ConcreteKind::Normal => match fc {
                ..=36.0 => 23.0,
                ..=48.0 => 23.5,
                ..=60.0 => 24.0,
                ..=80.0 => 24.5,
                ..=FC_MAX => 25.0,
                _ => unreachable!("Fc は生成のときに表の範囲に限っている"),
            },
        };
        UnitWeight::kn_per_m3(kn_per_m3)
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
