//! 必要スラブ厚さ Model。言葉は `docs/tools/slab-thickness/GLOSSARY.md`、スラブの形（Lx・Ly）は
//! `docs/slab/GLOSSARY.md`、コンクリートは `docs/materials/GLOSSARY.md`。
//! 保存するのは条件と検討点だけで、共有値・必要スラブ厚・決定条件は導出できるので `rustruct-calc` が求める。

use crate::concrete::Concrete;
use crate::quantity::{AreaLoad, Length};

/// 必要スラブ厚さ Model。すべての検討点に共通の条件と、検討点の並び。
/// 保存形式（serde）は、保存を実装するときに Kirchhoff 板 Model と合わせて付ける
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub conditions: Conditions,
    /// 検討点。並びは利用者が並べた順
    pub points: Vec<StudyPoint>,
}

/// すべての検討点に共通の条件。荷重・変形増大係数・クライテリアが読めるかは計算が答える
/// （条件欄を打ち直す途中を表せるように、生成では弾かない。Fc は [`Concrete`] が弾く）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conditions {
    pub concrete: Concrete,
    /// 仕上げ荷重 wf（スラブ自重を除く固定荷重）
    pub finish_load: AreaLoad,
    /// 積載荷重 wl（たわみ検討用）
    pub live_load: AreaLoad,
    /// 変形増大係数。弾性たわみ δe に乗じる
    pub amplification: f64,
    pub criteria: CriteriaSet,
}

/// クライテリア集合。有効な条件をすべて同時に満たす厚さが必要スラブ厚になる。
/// `None` と `false` は「その条件を判定に加えない」（条件欄の空欄、チェックのオフ）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CriteriaSet {
    /// 変形角（増大後たわみ / lx の上限。1/400 なら 0.0025）
    pub deflection_ratio: Option<f64>,
    /// たわみ絶対量（増大後たわみの上限。スパンに依らない）
    pub deflection_absolute: Option<Length>,
    /// 表18-1（周辺固定）の最小厚さを加えるか
    pub min_thickness: bool,
}

/// クライテリアの種類。決定条件として結果に載る。並びはクライテリア集合の並びで、
/// 必要厚さが同値のときに先に来る条件を決定条件とする
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Criterion {
    /// 変形角
    DeflectionRatio,
    /// たわみ絶対量
    DeflectionAbsolute,
    /// 表18-1 最小厚さ
    MinThickness,
}

/// 検討点。名称と (Lx, Ly) の組で表す、検討の対象とする1枚のスラブ。
#[derive(Debug, Clone, PartialEq)]
pub struct StudyPoint {
    pub name: String,
    pub spans: Spans,
}

/// スラブの x方向スパン Lx と y方向スパン Ly。どちらも内法（`docs/slab/adr/0001`）で、
/// 短辺・長辺のどちらであるかは問わない。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spans {
    pub x: Length,
    pub y: Length,
}
