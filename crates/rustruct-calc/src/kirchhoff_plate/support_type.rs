//! 支持条件の組を 21 種の型に正規化し、代表の向きから元の組への写し方を一つの値で持つ（ADR kirchhoff-plate-0003）。
//! 型ごとの解き方は代表の向き（型の 4 文字がそのまま x = 0、y = 0、x = Lx、y = Ly の辺）だけを知る。

use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};

/// 型の列挙と記号を一つの並びから作る。記号は GLOSSARY の約束どおり 4 文字の大文字
macro_rules! support_types {
    ($($name:ident),+ $(,)?) => {
        /// 支持条件の型。板を回転・裏返し、x と y を入れ替えて重なる組を同じものとした 21 種。
        /// 列挙子の名は型の記号そのもの（F < R < S の辞書順で最小の組）。
        #[allow(clippy::upper_case_acronyms, reason = "GLOSSARY が型を 4 文字の記号で書く")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum SupportType {
            $($name,)+
        }

        impl SupportType {
            pub const ALL: [SupportType; 21] = [$(SupportType::$name,)+];

            /// 型の記号（代表の向きの組の記号）
            pub fn symbol(self) -> &'static str {
                match self {
                    $(Self::$name => stringify!($name),)+
                }
            }
        }
    };
}

support_types! {
    FFFF, FFFR, FFFS, FFRR, FFRS, FFSS, FRFR, FRFS, FRRR, FRRS, FRSR,
    FRSS, FSFS, FSRS, FSSS, RRRR, RRRS, RRSS, RSRS, RSSS, SSSS,
}

impl SupportType {
    /// 利用者が選んだ組の型。組を回転・裏返し、x と y を入れ替えて、記号が最小になるもの
    pub fn of(supports: Supports) -> SupportType {
        Self::with_mapping(supports).0
    }

    /// 利用者が選んだ組の型と、代表の向きからその組への写し方。
    /// 8 通りの写し方で組を代表の向きに移し、記号が最小になるものを型とする。同じ記号なら先の写し方（恒等が最初）
    pub(crate) fn with_mapping(supports: Supports) -> (SupportType, Mapping) {
        let (mapping, representative) = Mapping::ALL
            .iter()
            .map(|&m| (m, m.representative_of(supports)))
            .min_by_key(|(_, r)| r.symbol())
            .expect("写し方は 8 通りある");
        let symbol = representative.symbol();
        let ty = Self::ALL
            .into_iter()
            .find(|t| t.symbol() == symbol)
            .expect("8 通りの写し方で最小にした記号は 21 種のどれか");
        (ty, mapping)
    }

    /// 板を支えられる型か。四辺自由（RRRR）と、単純支持 1 辺と自由 3 辺（RRRS）の 2 型だけが不安定で、
    /// ほかの 19 型は解ける
    pub fn is_stable(self) -> bool {
        !matches!(self, Self::RRRR | Self::RRRS)
    }

    /// 代表の向きの組。型の 4 文字がそのまま各辺の支持条件
    pub fn representative(self) -> Supports {
        let mut chars = self
            .symbol()
            .chars()
            .map(|c| Support::from_symbol(c).expect("型の記号は F・S・R"));
        let mut next = || chars.next().expect("型の記号は 4 文字");
        // Edge::ALL の順（x = 0 → y = 0 → x = Lx → y = Ly）
        let x0 = next();
        let y0 = next();
        let x_lx = next();
        let y_ly = next();
        Supports { x0, y0, x_lx, y_ly }
    }
}

/// 代表の向きと利用者の向きの間の写し方。利用者の座標を、先に x と y を入れ替え、次に x・y をそれぞれ
/// 裏返して代表の座標にする。結果の量の写し（Mx↔My、Qx↔Qy の入れ替えと、裏返しによる Q の符号）も
/// この値から決まり、型ごとの解き方には散らさない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Mapping {
    /// x と y を入れ替える（Lx と Ly も入れ替わる）
    pub transpose: bool,
    /// 入れ替えた後の x を裏返す（x = 0 の辺と x = Lx の辺が入れ替わる）
    pub flip_x: bool,
    /// 入れ替えた後の y を裏返す
    pub flip_y: bool,
}

impl Mapping {
    pub const IDENTITY: Mapping = Mapping {
        transpose: false,
        flip_x: false,
        flip_y: false,
    };

    /// 8 通りの写し方。恒等が最初で、同じ記号になる組は先のものを選ぶ
    pub const ALL: [Mapping; 8] = {
        let mut all = [Mapping::IDENTITY; 8];
        let mut i = 0;
        while i < 8 {
            all[i] = Mapping {
                transpose: i & 4 != 0,
                flip_x: i & 1 != 0,
                flip_y: i & 2 != 0,
            };
            i += 1;
        }
        all
    };

    /// 利用者の辺を代表の向きの辺に写す
    pub fn edge(self, edge: Edge) -> Edge {
        let edge = if self.transpose {
            match edge {
                Edge::X0 => Edge::Y0,
                Edge::Y0 => Edge::X0,
                Edge::XLx => Edge::YLy,
                Edge::YLy => Edge::XLx,
            }
        } else {
            edge
        };
        match edge {
            Edge::X0 if self.flip_x => Edge::XLx,
            Edge::XLx if self.flip_x => Edge::X0,
            Edge::Y0 if self.flip_y => Edge::YLy,
            Edge::YLy if self.flip_y => Edge::Y0,
            other => other,
        }
    }

    /// 利用者の隅を代表の向きの隅に写す
    pub fn corner(self, corner: Corner) -> Corner {
        let (ex, ey) = corner.edges();
        let (ex, ey) = (self.edge(ex), self.edge(ey));
        // 入れ替えで x の辺が y の辺になることがあるので、どちらが x かを見て組む
        let (x_edge, y_edge) = match ex {
            Edge::X0 | Edge::XLx => (ex, ey),
            Edge::Y0 | Edge::YLy => (ey, ex),
        };
        match (x_edge, y_edge) {
            (Edge::X0, Edge::Y0) => Corner::X0Y0,
            (Edge::XLx, Edge::Y0) => Corner::XLxY0,
            (Edge::XLx, Edge::YLy) => Corner::XLxYLy,
            (Edge::X0, Edge::YLy) => Corner::X0YLy,
            _ => unreachable!("辺の写しは x の辺と y の辺を 1 つずつ返す"),
        }
    }

    /// 代表の向きの組から、利用者の組へ。写しが元の組に戻ることを確かめるテストだけが使う
    #[cfg(test)]
    pub fn supports(self, representative: Supports) -> Supports {
        Supports {
            x0: representative.at(self.edge(Edge::X0)),
            y0: representative.at(self.edge(Edge::Y0)),
            x_lx: representative.at(self.edge(Edge::XLx)),
            y_ly: representative.at(self.edge(Edge::YLy)),
        }
    }

    /// 利用者の組を、この写し方で代表の向きに移した組
    fn representative_of(self, supports: Supports) -> Supports {
        let mut rep = supports;
        for edge in Edge::ALL {
            let value = supports.at(edge);
            match self.edge(edge) {
                Edge::X0 => rep.x0 = value,
                Edge::Y0 => rep.y0 = value,
                Edge::XLx => rep.x_lx = value,
                Edge::YLy => rep.y_ly = value,
            }
        }
        rep
    }
}

/// 写し方が座標と結果の量に効くところ。ここ以外で Lx↔Ly や符号を扱わない
impl Mapping {
    /// 代表の向きの板の辺長 (Lx', Ly')
    pub fn representative_lengths(self, lx: f64, ly: f64) -> (f64, f64) {
        if self.transpose { (ly, lx) } else { (lx, ly) }
    }

    /// 利用者の座標（隅が原点、mm）の点を代表の向きの板の座標（隅が原点）に
    pub fn representative_point(self, x: f64, y: f64, lx: f64, ly: f64) -> (f64, f64) {
        let (lx, ly) = self.representative_lengths(lx, ly);
        let (x, y) = if self.transpose { (y, x) } else { (x, y) };
        (
            if self.flip_x { lx - x } else { x },
            if self.flip_y { ly - y } else { y },
        )
    }

    /// 代表の向きの板の座標（隅が原点）の点を利用者の座標に。[`Self::representative_point`] の逆
    pub fn user_point(self, x: f64, y: f64, lx: f64, ly: f64) -> (f64, f64) {
        let (rep_lx, rep_ly) = self.representative_lengths(lx, ly);
        let x = if self.flip_x { rep_lx - x } else { x };
        let y = if self.flip_y { rep_ly - y } else { y };
        if self.transpose { (y, x) } else { (x, y) }
    }

    /// 利用者の辺の始点（座標 0 の端）からの距離を、代表の向きの辺の始点からの距離に
    pub fn representative_edge_position(
        self,
        edge: Edge,
        along: f64,
        lx: f64,
        ly: f64,
    ) -> (Edge, f64) {
        let (lx, ly) = self.representative_lengths(lx, ly);
        let rep = self.edge(edge);
        let along = match rep {
            Edge::X0 | Edge::XLx if self.flip_y => ly - along,
            Edge::Y0 | Edge::YLy if self.flip_x => lx - along,
            _ => along,
        };
        (rep, along)
    }

    /// 代表の向きで求めた点の応答を利用者の向きに戻す。x を裏返すと Qx の符号が、x と y を入れ替えると
    /// Mx と My、Qx と Qy が入れ替わる。たわみは変わらない
    pub fn point_response_back(self, r: RepresentativeResponse) -> RepresentativeResponse {
        let flipped = RepresentativeResponse {
            qx: if self.flip_x { -r.qx } else { r.qx },
            qy: if self.flip_y { -r.qy } else { r.qy },
            ..r
        };
        if self.transpose {
            RepresentativeResponse {
                mx: flipped.my,
                my: flipped.mx,
                qx: flipped.qy,
                qy: flipped.qx,
                ..flipped
            }
        } else {
            flipped
        }
    }

    /// 代表の向きの辺のせん断力（辺に直交する断面のもの）の符号を利用者の向きに戻す。
    /// 辺に直交する軸を裏返すと符号が変わる。曲げモーメントと反力は変わらない
    pub fn edge_shear_back(self, representative_edge: Edge, shear: f64) -> f64 {
        let flipped = match representative_edge {
            Edge::X0 | Edge::XLx => self.flip_x,
            Edge::Y0 | Edge::YLy => self.flip_y,
        };
        if flipped { -shear } else { shear }
    }
}

/// 代表の向きで求めた点の応答（内部の単位）。写し方で利用者の向きに戻す前の形
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepresentativeResponse {
    pub w: f64,
    pub mx: f64,
    pub my: f64,
    pub qx: f64,
    pub qy: f64,
}

/// 写し方の中を見るテスト。型への分類（81 通り → 21 型）は外から `tests/kirchhoff_plate_support_type.rs`、
/// 写しが結果の量に効くところは `solve` 越しに `tests/kirchhoff_plate_all_types.rs` が確かめる
#[cfg(test)]
mod tests {
    use super::*;

    const SUPPORTS: [Support; 3] = [Support::Fixed, Support::SimplySupported, Support::Free];

    fn every_combination() -> Vec<Supports> {
        let mut out = Vec::new();
        for x0 in SUPPORTS {
            for y0 in SUPPORTS {
                for x_lx in SUPPORTS {
                    for y_ly in SUPPORTS {
                        out.push(Supports { x0, y0, x_lx, y_ly });
                    }
                }
            }
        }
        out
    }

    #[test]
    fn the_representative_normalizes_with_the_identity() {
        for ty in SupportType::ALL {
            let (again, mapping) = SupportType::with_mapping(ty.representative());
            assert_eq!(again, ty);
            assert_eq!(mapping, Mapping::IDENTITY);
        }
    }

    #[test]
    fn the_mapping_carries_the_representative_back_to_the_chosen_supports() {
        for s in every_combination() {
            let (ty, mapping) = SupportType::with_mapping(s);
            assert_eq!(mapping.supports(ty.representative()), s, "{}", s.symbol());
            // 辺の写しと組の写しは同じことを言う
            for edge in Edge::ALL {
                assert_eq!(
                    s.at(edge),
                    ty.representative().at(mapping.edge(edge)),
                    "{} の {edge}",
                    s.symbol()
                );
            }
            // 隅は会う 2 辺の写しで決まる
            for corner in Corner::ALL {
                let (ex, ey) = corner.edges();
                let mapped = mapping.corner(corner).edges();
                let pair = [mapping.edge(ex), mapping.edge(ey)];
                assert!(pair.contains(&mapped.0) && pair.contains(&mapped.1));
            }
        }
    }

    #[test]
    fn symmetric_combinations_normalize_with_the_identity() {
        for support in SUPPORTS {
            let (_, mapping) = SupportType::with_mapping(Supports::all(support));
            assert_eq!(mapping, Mapping::IDENTITY);
        }
    }

    #[test]
    fn a_rotated_combination_maps_edges_and_positions() {
        // 固定 1 辺が x = Lx にある組。代表 FRRR は固定を x = 0 に置くので、x を裏返す写し
        let s = Supports {
            x0: Support::Free,
            y0: Support::Free,
            x_lx: Support::Fixed,
            y_ly: Support::Free,
        };
        let (ty, mapping) = SupportType::with_mapping(s);
        assert_eq!(ty.symbol(), "FRRR");
        assert_eq!(mapping.edge(Edge::XLx), Edge::X0);
        assert_eq!(mapping.edge(Edge::Y0), Edge::Y0);
        assert!(mapping.flip_x && !mapping.flip_y && !mapping.transpose);

        // 固定 1 辺が y = 0 にある組。x と y を入れ替える写し
        let s = Supports {
            x0: Support::Free,
            y0: Support::Fixed,
            x_lx: Support::Free,
            y_ly: Support::Free,
        };
        let (ty, mapping) = SupportType::with_mapping(s);
        assert_eq!(ty.symbol(), "FRRR");
        assert_eq!(mapping.edge(Edge::Y0), Edge::X0);
        assert!(mapping.transpose && !mapping.flip_x && !mapping.flip_y);
    }
}
