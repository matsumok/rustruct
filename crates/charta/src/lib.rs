//! グラフ描画。rustruct 固有のものは置かない。役割と依存の向きは `docs/stack.md`。
//!
//! 最初の一片は分布図（等値線と帯の図）。継ぎ目は 4 つ:
//!
//! 1. [`Grid`] → [`contour`]`(grid, levels)` → 等値線 [`Isoline`] と帯 [`Band`]（純粋関数）
//! 2. [`levels`]: 値の範囲と段数の目安から、1・2・2.5・5 × 10ⁿ に丸めた帯の境
//! 3. [`Scene`]（パス・矩形・円・文字。色は役割名）→ [`to_svg`]`(scene, theme, colors)` → SVG の文字列。
//!    色の書き方 [`Colors`] は 2 つ: 値（`#rrggbb`）と、CSS 変数（`var(--color-ink)`）
//! 4. [`distribution`]: 1〜3 を ADR site-0011 で決めた見た目でつなぐ
//!
//! 二片目は、水準の境がグラフ y = g(x) で与えられる場の図（必要スラブ厚の等厚曲線）:
//!
//! 5. [`Plot`]: データの矩形と画面の矩形の対応。[`axes`] が枠・内向きの目盛り・軸の名前を描く
//! 6. [`level_graphs`]: 境のグラフ [`LevelGraph`] から、分布図と同じ見た目の帯・線・数値
//!
//! 色と書体の値は charta に無い。呼び手が役割名と値の表 [`Theme`] を作って渡す。

pub mod color;
pub mod contour;
pub mod distribution;
pub mod grid;
mod label;
pub mod level_graphs;
pub mod levels;
pub mod palette;
pub mod plot;
pub mod scene;
pub mod svg;
pub mod theme;

pub use contour::{Band, Isoline, contour};
pub use distribution::distribution;
pub use grid::{Grid, GridError};
pub use label::HALO_WIDTH;
pub use level_graphs::{LevelGraph, level_graphs};
pub use palette::Palette;
pub use plot::{Plot, axes};
pub use scene::{Anchor, Font, Item, Paint, Pt, Role, Scene, Stroke};
pub use svg::{Colors, to_svg};
pub use theme::Theme;

/// シーンを描けなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// テーマに無い役割名。
    UnknownRole(String),
    /// 帯の色の列の段数が足りない（`seq` は 2 段、`div` は 4 段から）か、`div` が偶数でない。
    Stops { prefix: String, count: usize },
    /// 色の値が `#rrggbb` でなく、間の色を作れない。
    NotHex(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownRole(role) => write!(f, "テーマに無い役割名: {role}"),
            Error::Stops { prefix, count } => {
                write!(f, "帯の色の列 {prefix} の段数が使えない: {count}")
            }
            Error::NotHex(value) => write!(f, "#rrggbb でない色: {value}"),
        }
    }
}

impl std::error::Error for Error {}
