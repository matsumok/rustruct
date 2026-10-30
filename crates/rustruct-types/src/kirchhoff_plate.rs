//! Kirchhoff 板 Model と、支持条件の組、辺・隅の呼び方。言葉は `docs/tools/kirchhoff-plate/GLOSSARY.md`。
//! 支持条件の組は利用者が辺ごとに選ぶ値で Model に保存する。型（21 種）と写し方は導出できるので
//! ここには無く、`rustruct-calc` が求める（ADR-0003）。

use std::fmt;

use crate::quantity::{AreaLoad, Length, PoissonRatio, Stress};

/// Kirchhoff 板 Model。1 枚の板の入力で、保存するのはこの値だけ（`docs/tools/kirchhoff-plate/adr/0001`）。
/// 保存形式（serde）は、保存を実装するときに、サイトの ADR-0004（量の型）が予告した保存形式の見直しと合わせて付ける
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plate {
    /// x方向の辺長 Lx
    pub lx: Length,
    /// y方向の辺長 Ly
    pub ly: Length,
    /// 板厚 t
    pub thickness: Length,
    /// ヤング係数 E
    pub young_modulus: Stress,
    /// ポアソン比 ν
    pub poisson_ratio: PoissonRatio,
    /// 等分布荷重 p（下向き正）
    pub load: AreaLoad,
    /// 4 辺の支持条件の組
    pub supports: Supports,
}

/// 1 つの辺の支持条件。記号は F・S・R。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Support {
    /// 固定: たわみと、辺に直交する方向の傾きがともに 0
    Fixed,
    /// 単純支持: たわみと、辺に直交する方向の曲げモーメントがともに 0
    SimplySupported,
    /// 自由: 辺に直交する方向の曲げモーメントと辺の反力がともに 0
    Free,
}

impl Support {
    /// 記号。型の 4 文字はこの記号を F < R < S の順で比べる
    pub fn symbol(self) -> char {
        match self {
            Self::Fixed => 'F',
            Self::SimplySupported => 'S',
            Self::Free => 'R',
        }
    }

    /// 記号から。F・S・R 以外は `None`
    pub fn from_symbol(symbol: char) -> Option<Self> {
        match symbol {
            'F' => Some(Self::Fixed),
            'S' => Some(Self::SimplySupported),
            'R' => Some(Self::Free),
            _ => None,
        }
    }
}

/// 板の辺。座標で呼ぶ（画面の上下左右は使わない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    /// x = 0 の辺
    X0,
    /// x = Lx の辺
    XLx,
    /// y = 0 の辺
    Y0,
    /// y = Ly の辺
    YLy,
}

impl Edge {
    /// 記号の並びの順（x = 0 → y = 0 → x = Lx → y = Ly、反時計回り）
    pub const ALL: [Edge; 4] = [Edge::X0, Edge::Y0, Edge::XLx, Edge::YLy];
}

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::X0 => "x = 0",
            Self::XLx => "x = Lx",
            Self::Y0 => "y = 0",
            Self::YLy => "y = Ly",
        })
    }
}

/// 板の隅。会う 2 辺で呼ぶ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Corner {
    X0Y0,
    XLxY0,
    XLxYLy,
    X0YLy,
}

impl Corner {
    /// 原点の隅から反時計回り
    pub const ALL: [Corner; 4] = [Corner::X0Y0, Corner::XLxY0, Corner::XLxYLy, Corner::X0YLy];

    /// 隅で会う x の辺と y の辺
    pub fn edges(self) -> (Edge, Edge) {
        match self {
            Self::X0Y0 => (Edge::X0, Edge::Y0),
            Self::XLxY0 => (Edge::XLx, Edge::Y0),
            Self::XLxYLy => (Edge::XLx, Edge::YLy),
            Self::X0YLy => (Edge::X0, Edge::YLy),
        }
    }
}

/// 4 辺の支持条件の組。不安定な組（四辺自由など）も作れる。画面で辺を 1 つずつ選ぶ途中を表すためで、
/// 解けるかどうかは計算が答える（ADR-0003）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Supports {
    pub x0: Support,
    pub y0: Support,
    pub x_lx: Support,
    pub y_ly: Support,
}

impl Supports {
    /// 4 辺とも同じ支持条件
    pub const fn all(support: Support) -> Self {
        Self {
            x0: support,
            y0: support,
            x_lx: support,
            y_ly: support,
        }
    }

    pub fn at(self, edge: Edge) -> Support {
        match edge {
            Edge::X0 => self.x0,
            Edge::Y0 => self.y0,
            Edge::XLx => self.x_lx,
            Edge::YLy => self.y_ly,
        }
    }

    /// 4 文字の記号。x = 0 の辺から反時計回り（内部の約束で、画面には出さない）
    pub fn symbol(self) -> String {
        Edge::ALL.iter().map(|&e| self.at(e).symbol()).collect()
    }
}
