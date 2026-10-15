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
