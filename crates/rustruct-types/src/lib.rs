//! 共有型。物理量（ADR site-0004）、材料とその性質（`docs/materials/`）、Model と保存形式（ADR site-0003）。
//! 役割と依存の向きは `docs/stack.md`。

pub mod allowable_stress;
pub mod concrete;
pub mod kirchhoff_plate;
pub mod quantity;
pub mod rebar;
pub mod slab_thickness;
