//! グラフ描画。rustruct 固有のものは置かない。役割と依存の向きは `docs/stack.md`。
//!
//! 最初の一片は分布図（等値線と帯の図）。継ぎ目は 4 つ:
//!
//! 1. [`Grid`] → [`contour`]`(grid, levels)` → 等値線 [`Isoline`] と帯 [`Band`]（純粋関数）
//! 2. [`levels`]: 値の範囲と段数の目安から、1・2・2.5・5 × 10ⁿ に丸めた帯の境
//! 3. [`Scene`]（パス・矩形・文字。色は役割名）→ [`to_svg`]`(scene, theme)` → SVG の文字列
//! 4. [`distribution`]: 1〜3 を `docs/adr/0011` で決めた見た目でつなぐ
//!
//! 色と書体の値は `docs/design/tokens.json` にだけあり、[`Theme`] がそこから引く。

pub mod color;
pub mod contour;
pub mod distribution;
pub mod grid;
pub mod levels;
pub mod palette;
pub mod scene;
pub mod svg;
pub mod theme;

pub use contour::{Band, Isoline, contour};
pub use distribution::distribution;
pub use grid::{Grid, GridError};
pub use palette::Palette;
pub use scene::{Anchor, Font, Item, Paint, Pt, Role, Scene, Stroke};
pub use svg::to_svg;
pub use theme::{Mode, Theme};

/// シーンを描けなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// tokens.json に無い役割名。
    UnknownRole(String),
    /// 色の値が `#rrggbb` でなく、間の色を作れない。
    NotHex(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownRole(role) => write!(f, "tokens.json に無い役割名: {role}"),
            Error::NotHex(value) => write!(f, "#rrggbb でない色: {value}"),
        }
    }
}

impl std::error::Error for Error {}
