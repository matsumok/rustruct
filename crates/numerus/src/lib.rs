//! 表と図に書く数の書き方。rustruct 固有のものは置かない。決め事は `docs/adr/0013`。
//!
//! 等幅の文字が前提。継ぎ目は 2 つ:
//!
//! 1. [`significant`]・[`fixed`]・[`at_place`]: f64 → 文字列。0.001 ≦ |x| < 1,000,000 は小数で、
//!    整数部を 3 桁ごとに「,」で区切る。外は E（1.23E+06、-1.52E-06）
//! 2. [`point_pads`]: 列の値の並び → 小数点を揃える右の埋め（数字幅の空白 U+2007 の数）

mod align;
mod notation;

pub use align::{FIGURE_SPACE, point_pads};
pub use notation::{at_place, fixed, significant};
