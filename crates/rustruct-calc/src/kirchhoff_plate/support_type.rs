//! 支持条件の組を 21 種の型に正規化し、代表の向きから元の組への写し方を一つの値で持つ（ADR-0003）。
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
    /// 利用者が選んだ組の型と、代表の向きからその組への写し方。
    /// 8 通りの写し方で組を代表の向きに移し、記号が最小になるものを型とする。同じ記号なら先の写し方（恒等が最初）
    pub fn of(supports: Supports) -> (SupportType, Mapping) {
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
pub struct Mapping {
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

    /// 代表の向きの組から、利用者の組へ
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
