//! 物理量の newtype（ADR-0004）。中身は N・mm・s の系（質量は t）に正規化した `f64`。
//! 作るときと取り出すときに単位を名前で言う。型は使う Tool が出たときに要る分だけ足す。

macro_rules! quantity {
    ($(#[$doc:meta])* $name:ident, $internal:literal) => {
        $(#[$doc])*
        #[doc = concat!("\n\n内部の単位は ", $internal, "。")]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct $name(f64);
    };
}

quantity!(
    /// 長さ。
    Length, "mm"
);

impl Length {
    pub const fn mm(value: f64) -> Self {
        Self(value)
    }
    pub fn as_mm(self) -> f64 {
        self.0
    }
}

quantity!(
    /// 面積。
    Area, "mm²"
);

impl Area {
    pub const fn mm2(value: f64) -> Self {
        Self(value)
    }
    pub fn as_mm2(self) -> f64 {
        self.0
    }
}

quantity!(
    /// 単位長さあたりの質量（鉄筋の単位質量など）。
    LinearMass, "t/mm"
);

impl LinearMass {
    /// 1 kg/m = 10⁻³ t / 10³ mm
    pub const fn kg_per_m(value: f64) -> Self {
        Self(value / 1e6)
    }
    pub fn as_kg_per_m(self) -> f64 {
        self.0 * 1e6
    }
}

quantity!(
    /// 応力度・強度・ヤング係数。
    Stress, "N/mm²"
);

impl Stress {
    pub const fn n_per_mm2(value: f64) -> Self {
        Self(value)
    }
    pub fn as_n_per_mm2(self) -> f64 {
        self.0
    }
}

quantity!(
    /// 単位体積重量。
    UnitWeight, "N/mm³"
);

impl UnitWeight {
    /// 1 kN/m³ = 10³ N / 10⁹ mm³
    pub const fn kn_per_m3(value: f64) -> Self {
        Self(value / 1e6)
    }
    pub fn as_kn_per_m3(self) -> f64 {
        self.0 * 1e6
    }
}

quantity!(
    /// 単位面積あたりの荷重（Kirchhoff 板の等分布荷重）。次元は [`Stress`] と同じだが意味が違うので型を分ける。
    AreaLoad, "N/mm²"
);

impl AreaLoad {
    /// 1 kN/m² = 10³ N / 10⁶ mm²
    pub const fn kn_per_m2(value: f64) -> Self {
        Self(value * 1e-3)
    }
    pub fn as_kn_per_m2(self) -> f64 {
        self.0 * 1e3
    }
    pub const fn n_per_mm2(value: f64) -> Self {
        Self(value)
    }
    pub fn as_n_per_mm2(self) -> f64 {
        self.0
    }
}

quantity!(
    /// 単位幅あたりの曲げモーメント（板の Mx・My）。
    MomentPerLength, "N·mm/mm"
);

impl MomentPerLength {
    /// 1 kN·m/m = 10³ N × 10³ mm / 10³ mm
    pub const fn knm_per_m(value: f64) -> Self {
        Self(value * 1e3)
    }
    pub fn as_knm_per_m(self) -> f64 {
        self.0 * 1e-3
    }
    pub const fn nmm_per_mm(value: f64) -> Self {
        Self(value)
    }
    pub fn as_nmm_per_mm(self) -> f64 {
        self.0
    }
}

quantity!(
    /// 単位長さあたりの力（板のせん断力と辺の反力）。せん断力と反力は名前で区別し、型は分けない。
    ForcePerLength, "N/mm"
);

impl ForcePerLength {
    /// 1 kN/m = 10³ N / 10³ mm
    pub const fn kn_per_m(value: f64) -> Self {
        Self(value)
    }
    pub fn as_kn_per_m(self) -> f64 {
        self.0
    }
    pub const fn n_per_mm(value: f64) -> Self {
        Self(value)
    }
    pub fn as_n_per_mm(self) -> f64 {
        self.0
    }
}

quantity!(
    /// 集中する力（板の隅反力）。
    Force, "N"
);

impl Force {
    pub const fn kn(value: f64) -> Self {
        Self(value * 1e3)
    }
    pub fn as_kn(self) -> f64 {
        self.0 * 1e-3
    }
    pub const fn n(value: f64) -> Self {
        Self(value)
    }
    pub fn as_n(self) -> f64 {
        self.0
    }
}

/// ポアソン比。無次元。0 以上 0.5 未満の範囲は生成で守る（`docs/tools/kirchhoff-plate/adr/0005`）。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct PoissonRatio(f64);

/// ポアソン比を作れない理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoissonRatioError {
    /// 0 以上 0.5 未満の有限値でない
    OutOfRange,
}

impl std::fmt::Display for PoissonRatioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange => f.write_str("ポアソン比が 0 以上 0.5 未満でない"),
        }
    }
}

impl std::error::Error for PoissonRatioError {}

impl PoissonRatio {
    pub fn new(value: f64) -> Result<Self, PoissonRatioError> {
        // NaN は両方の比較で偽になり、ここで弾かれる
        if (0.0..0.5).contains(&value) {
            Ok(Self(value))
        } else {
            Err(PoissonRatioError::OutOfRange)
        }
    }
    pub fn value(self) -> f64 {
        self.0
    }
}
