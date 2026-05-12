use serde::{Deserialize, Serialize};

/// H形鋼断面（上下非対称対応）。寸法単位はすべて mm。
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteelSectionH {
    name: String,
    h: f64,    // 断面高さ
    b_t: f64,  // 上端フランジ幅
    b_b: f64,  // 下端フランジ幅
    tw: f64,   // ウェブ厚さ
    tf_t: f64, // 上端フランジ厚さ
    tf_b: f64, // 下端フランジ厚さ
    r_t: f64,  // 上端フィレット半径
    r_b: f64,  // 下端フィレット半径
}

impl SteelSectionH {
    /// 対称H形断面を生成（上下同一フランジ・フィレット）
    pub fn new(name: impl Into<String>, h: f64, b: f64, tw: f64, tf: f64, r: f64) -> Self {
        Self {
            name: name.into(),
            h,
            b_t: b,
            b_b: b,
            tw,
            tf_t: tf,
            tf_b: tf,
            r_t: r,
            r_b: r,
        }
    }

    /// 非対称H形断面を生成
    pub fn new_asymmetric(
        name: impl Into<String>,
        h: f64,
        b_t: f64,
        b_b: f64,
        tw: f64,
        tf_t: f64,
        tf_b: f64,
        r_t: f64,
        r_b: f64,
    ) -> Self {
        Self { name: name.into(), h, b_t, b_b, tw, tf_t, tf_b, r_t, r_b }
    }

    // ---- ゲッター ----

    pub fn name(&self) -> &str { &self.name }
    pub fn h(&self) -> f64 { self.h }
    pub fn b_t(&self) -> f64 { self.b_t }
    pub fn b_b(&self) -> f64 { self.b_b }
    pub fn tw(&self) -> f64 { self.tw }
    pub fn tf_t(&self) -> f64 { self.tf_t }
    pub fn tf_b(&self) -> f64 { self.tf_b }
    pub fn r_t(&self) -> f64 { self.r_t }
    pub fn r_b(&self) -> f64 { self.r_b }

    // ---- 断面性能（フィレット面積は無視した近似値）----

    /// ウェブ高さ（フランジ外面間）
    fn hw(&self) -> f64 {
        self.h - self.tf_t - self.tf_b
    }

    /// 断面積 (mm²)
    pub fn area(&self) -> f64 {
        self.b_t * self.tf_t + self.b_b * self.tf_b + self.hw() * self.tw
    }

    /// 図心位置（断面下端からの距離, mm）
    pub fn centroid_y(&self) -> f64 {
        let a_top = self.b_t * self.tf_t;
        let a_web = self.hw() * self.tw;
        let a_bot = self.b_b * self.tf_b;

        let y_top = self.h - self.tf_t / 2.0;
        let y_web = self.tf_b + self.hw() / 2.0;
        let y_bot = self.tf_b / 2.0;

        (a_top * y_top + a_web * y_web + a_bot * y_bot) / self.area()
    }

    /// 強軸断面二次モーメント (mm⁴)
    pub fn ix(&self) -> f64 {
        let yg = self.centroid_y();

        let y_top = self.h - self.tf_t / 2.0;
        let i_top = self.b_t * self.tf_t.powi(3) / 12.0
            + self.b_t * self.tf_t * (y_top - yg).powi(2);

        let y_web = self.tf_b + self.hw() / 2.0;
        let i_web = self.tw * self.hw().powi(3) / 12.0
            + self.tw * self.hw() * (y_web - yg).powi(2);

        let y_bot = self.tf_b / 2.0;
        let i_bot = self.b_b * self.tf_b.powi(3) / 12.0
            + self.b_b * self.tf_b * (y_bot - yg).powi(2);

        i_top + i_web + i_bot
    }

    /// 弱軸断面二次モーメント (mm⁴)
    pub fn iy(&self) -> f64 {
        self.tf_t * self.b_t.powi(3) / 12.0
            + self.hw() * self.tw.powi(3) / 12.0
            + self.tf_b * self.b_b.powi(3) / 12.0
    }

    /// 上端断面係数 (mm³)
    pub fn zx_top(&self) -> f64 {
        self.ix() / (self.h - self.centroid_y())
    }

    /// 下端断面係数 (mm³)
    pub fn zx_bot(&self) -> f64 {
        self.ix() / self.centroid_y()
    }

    /// 弱軸断面係数（幅の大きい方のフランジ基準, mm³）
    pub fn zy(&self) -> f64 {
        self.iy() / (self.b_t.max(self.b_b) / 2.0)
    }

    /// せん断有効断面積（ウェブのみ, mm²）
    pub fn shear_area(&self) -> f64 {
        self.hw() * self.tw
    }
}

/// FEM 梁要素が必要とする断面性能の集約値。単位は mm 系。
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionProps {
    pub a: f64,  // 断面積 (mm²)
    pub ix: f64, // 強軸断面二次モーメント (mm⁴)
    pub iy: f64, // 弱軸断面二次モーメント (mm⁴)
    pub av: f64, // せん断有効断面積 (mm²)
}

impl From<&SteelSectionH> for SectionProps {
    fn from(s: &SteelSectionH) -> Self {
        Self {
            a: s.area(),
            ix: s.ix(),
            iy: s.iy(),
            av: s.shear_area(),
        }
    }
}
