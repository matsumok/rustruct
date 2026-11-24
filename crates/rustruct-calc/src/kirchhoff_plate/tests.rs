//! 入口（[`Solution`]）が、利用者の組を代表の向きの板の解の写しとして返すことを、有限要素で解かずに確かめる。
//! 解き方はテスト用の adapter（[`Polynomial`]）で、どの対称でも自分自身に重ならない多項式のたわみの場を返す。
//! 写しの期待値は計算核の写し方（`Mapping`）を使わず、ここで入れ替えと裏返しを書いて求める（[`Symmetry`]。
//! `tests/kirchhoff_plate_all_types.rs` と同じ考え）。`solve` が換算の値を正しく組むことの通しの確かめ、
//! 隅反力と辺の曲げモーメントの構造上の 0 は、`solve` 越しのテストが見る

use rustruct_types::kirchhoff_plate::{Corner, Edge, Supports};
use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};

use super::solver::{Derivatives, EdgePoint, Solver};
use super::{Plate, Solution, SupportType, normalize};

/// x^i y^j の係数 (i, j, c)。係数をすべて違えて、x・y の裏返しにも入れ替えにも重ならないようにする。
/// 4 次の項があるので 3 階の導関数も場所で変わる
const TERMS: [(i32, i32, f64); 15] = [
    (0, 0, 1.0),
    (1, 0, 0.31),
    (0, 1, -0.47),
    (2, 0, 0.53),
    (1, 1, 0.29),
    (0, 2, -0.61),
    (3, 0, 0.17),
    (2, 1, -0.23),
    (1, 2, 0.37),
    (0, 3, 0.41),
    (4, 0, -0.11),
    (3, 1, 0.13),
    (2, 2, -0.19),
    (1, 3, 0.07),
    (0, 4, 0.05),
];

/// テスト用の解き方。代表の向きの板（中央原点、y の半辺長 b）の上の多項式のたわみの場
struct Polynomial {
    b: f64,
}

/// n (n − 1) … (n − k + 1)
fn falling(n: i32, k: i32) -> f64 {
    (0..k).map(|m| f64::from(n - m)).product()
}

impl Polynomial {
    /// x で `dx` 回、y で `dy` 回微分した値
    fn derivative(&self, dx: i32, dy: i32, x: f64, y: f64) -> f64 {
        TERMS
            .iter()
            .filter(|&&(i, j, _)| i >= dx && j >= dy)
            .map(|&(i, j, c)| c * falling(i, dx) * falling(j, dy) * x.powi(i - dx) * y.powi(j - dy))
            .sum()
    }

    fn derivatives(&self, x: f64, y: f64) -> Derivatives {
        let d = |dx, dy| self.derivative(dx, dy, x, y);
        Derivatives {
            w: d(0, 0),
            w_xx: d(2, 0),
            w_yy: d(0, 2),
            w_xy: d(1, 1),
            w_xxx: d(3, 0),
            w_xxy: d(2, 1),
            w_xyy: d(1, 2),
            w_yyy: d(0, 3),
        }
    }

    /// 代表の向きの辺の上の、辺に沿う座標 `along` の点
    fn edge_point(&self, edge: Edge, along: f64) -> (f64, f64) {
        match edge {
            Edge::X0 => (-1.0, along),
            Edge::XLx => (1.0, along),
            Edge::Y0 => (along, -self.b),
            Edge::YLy => (along, self.b),
        }
    }
}

impl Solver for Polynomial {
    fn interior(&self, x: f64, y: f64) -> Derivatives {
        self.derivatives(x, y)
    }

    /// 辺の反力は 1 階の導関数を混ぜた値。辺に沿って向きの区別がつけばよい
    fn edge(&self, edge: Edge, along: f64) -> EdgePoint {
        let (x, y) = self.edge_point(edge, along);
        EdgePoint {
            derivatives: self.derivatives(x, y),
            reaction: self.derivative(1, 0, x, y) + 2.0 * self.derivative(0, 1, x, y),
        }
    }

    /// 辺ごとに違う値
    fn edge_reaction_total(&self, edge: Edge) -> f64 {
        match edge {
            Edge::X0 => 1.1,
            Edge::Y0 => 2.3,
            Edge::XLx => 3.7,
            Edge::YLy => 5.3,
        }
    }

    fn corner_w_xy(&self, corner: Corner) -> f64 {
        let (x, y) = match corner {
            Corner::X0Y0 => (-1.0, -self.b),
            Corner::XLxY0 => (1.0, -self.b),
            Corner::XLxYLy => (1.0, self.b),
            Corner::X0YLy => (-1.0, self.b),
        };
        self.derivative(1, 1, x, y)
    }

    /// x = Lx の辺の上の点。帰りの換算で板の外に出れば、入口が点の応答を取れずに落ちる
    fn deflection_maximum(&self) -> (f64, f64) {
        (1.0, 0.3 * self.b)
    }
}

/// 支持条件の組 `supports` の板を、テスト用の解き方で組んだ解。ν = 0.2、p = 10 kN/m²
fn solve_with_polynomial(lx: f64, ly: f64, supports: Supports) -> Solution {
    let plate = Plate {
        lx: Length::mm(lx),
        ly: Length::mm(ly),
        thickness: Length::mm(100.0),
        young_modulus: Stress::n_per_mm2(20_000.0),
        poisson_ratio: PoissonRatio::new(0.2).unwrap(),
        load: AreaLoad::kn_per_m2(10.0),
        supports,
    };
    let (support_type, frame) = normalize(&plate).expect("解ける組");
    Solution {
        plate,
        support_type,
        frame,
        solver: Box::new(Polynomial { b: frame.b() }),
    }
}

/// 長方形の 8 つの対称。利用者の板の座標を、先に x と y を入れ替え、次に x・y をそれぞれ裏返して代表の向きの
/// 板の座標にする。並びは恒等が最初で、x の裏返し、y の裏返し、入れ替えの順に 1・2・4 の桁
#[derive(Debug, Clone, Copy)]
struct Symmetry {
    transpose: bool,
    flip_x: bool,
    flip_y: bool,
}

const SYMMETRIES: [Symmetry; 8] = {
    let mut all = [Symmetry {
        transpose: false,
        flip_x: false,
        flip_y: false,
    }; 8];
    let mut i = 0;
    while i < 8 {
        all[i] = Symmetry {
            transpose: i & 4 != 0,
            flip_x: i & 1 != 0,
            flip_y: i & 2 != 0,
        };
        i += 1;
    }
    all
};

impl Symmetry {
    /// 利用者の辺 → 代表の向きの板の辺
    fn edge(self, edge: Edge) -> Edge {
        let edge = match (self.transpose, edge) {
            (false, e) => e,
            (true, Edge::X0) => Edge::Y0,
            (true, Edge::Y0) => Edge::X0,
            (true, Edge::XLx) => Edge::YLy,
            (true, Edge::YLy) => Edge::XLx,
        };
        match edge {
            Edge::X0 if self.flip_x => Edge::XLx,
            Edge::XLx if self.flip_x => Edge::X0,
            Edge::Y0 if self.flip_y => Edge::YLy,
            Edge::YLy if self.flip_y => Edge::Y0,
            e => e,
        }
    }

    /// 代表の向きの組から、利用者の組へ
    fn supports(self, representative: Supports) -> Supports {
        Supports {
            x0: representative.at(self.edge(Edge::X0)),
            y0: representative.at(self.edge(Edge::Y0)),
            x_lx: representative.at(self.edge(Edge::XLx)),
            y_ly: representative.at(self.edge(Edge::YLy)),
        }
    }

    /// 代表の向きの板の辺長から利用者の板の辺長へ（逆も同じ）
    fn lengths(self, lx: f64, ly: f64) -> (f64, f64) {
        if self.transpose { (ly, lx) } else { (lx, ly) }
    }

    /// 利用者の点（利用者の板の辺長 lx・ly）→ 代表の向きの板の点
    fn point(self, x: f64, y: f64, lx: f64, ly: f64) -> (f64, f64) {
        let (x, y, lx, ly) = if self.transpose {
            (y, x, ly, lx)
        } else {
            (x, y, lx, ly)
        };
        (
            if self.flip_x { lx - x } else { x },
            if self.flip_y { ly - y } else { y },
        )
    }

    /// 代表の向きの板の点（代表の向きの板の辺長 lx・ly）→ 利用者の点。[`Self::point`] の逆
    fn point_back(self, x: f64, y: f64, lx: f64, ly: f64) -> (f64, f64) {
        let x = if self.flip_x { lx - x } else { x };
        let y = if self.flip_y { ly - y } else { y };
        if self.transpose { (y, x) } else { (x, y) }
    }

    /// 利用者の x・y の向きが、代表の向きの板で裏返るか。裏返る向きの微分を含むせん断力は符号が変わる
    fn x_reversed(self) -> bool {
        if self.transpose {
            self.flip_y
        } else {
            self.flip_x
        }
    }
    fn y_reversed(self) -> bool {
        if self.transpose {
            self.flip_x
        } else {
            self.flip_y
        }
    }
}

fn sign(reversed: bool) -> f64 {
    if reversed { -1.0 } else { 1.0 }
}

fn corner_at(x: f64, y: f64) -> Corner {
    match (x == 0.0, y == 0.0) {
        (true, true) => Corner::X0Y0,
        (false, true) => Corner::XLxY0,
        (false, false) => Corner::XLxYLy,
        (true, false) => Corner::X0YLy,
    }
}

/// 同じ解き方の値を座標の換算を変えて取った差の許容（丸め）
const ROUND_OFF: f64 = 1e-9;

/// 量ごとの大きさ。代表の向きの板のグリッドの点の最大
struct Scale {
    w: f64,
    m: f64,
    q: f64,
}

impl Scale {
    fn of(s: &Solution) -> Scale {
        let grid = s.grid(8, 8);
        let max = |f: fn(&super::PointResponse) -> f64| {
            grid.iter().map(|p| f(p).abs()).fold(0.0, f64::max)
        };
        Scale {
            w: max(|p| p.w.as_mm()),
            m: max(|p| p.mx.as_nmm_per_mm().abs().max(p.my.as_nmm_per_mm().abs())),
            q: max(|p| p.qx.as_n_per_mm().abs().max(p.qy.as_n_per_mm().abs())),
        }
    }
}

/// 利用者の組の解 `user` が、代表の向きの組の解 `base` を対称 `sym` で写したものになっていること。点の応答、
/// 辺に沿った応答、辺の反力の合計、隅反力、たわみの最大。たわみは変わらず、入れ替えで Mx↔My・Qx↔Qy、裏返す
/// 向きのせん断力は符号が変わる
fn assert_maps(user: &Solution, base: &Solution, sym: Symmetry, what: &str) {
    let scale = Scale::of(base);
    let (lx, ly) = (user.plate().lx.as_mm(), user.plate().ly.as_mm());
    let (base_lx, base_ly) = (base.plate().lx.as_mm(), base.plate().ly.as_mm());
    let close = |a: f64, b: f64, unit: f64, name: &str| {
        let tolerance = ROUND_OFF * unit.max(b.abs());
        assert!(
            (a - b).abs() <= tolerance,
            "{what} の {name}: {a} と {b} の差が許容 {tolerance} を超える"
        );
    };
    for p in user.grid(8, 8) {
        let (x, y) = (p.x.as_mm(), p.y.as_mm());
        let (bx, by) = sym.point(x, y, lx, ly);
        let o = base.at(Length::mm(bx), Length::mm(by)).expect("板の上の点");
        let (omx, omy, oqx, oqy) = if sym.transpose {
            (o.my, o.mx, o.qy, o.qx)
        } else {
            (o.mx, o.my, o.qx, o.qy)
        };
        let point = format!("({x}, {y})");
        close(p.w.as_mm(), o.w.as_mm(), scale.w, &format!("{point} の w"));
        close(
            p.mx.as_nmm_per_mm(),
            omx.as_nmm_per_mm(),
            scale.m,
            &format!("{point} の Mx"),
        );
        close(
            p.my.as_nmm_per_mm(),
            omy.as_nmm_per_mm(),
            scale.m,
            &format!("{point} の My"),
        );
        close(
            p.qx.as_n_per_mm(),
            sign(sym.x_reversed()) * oqx.as_n_per_mm(),
            scale.q,
            &format!("{point} の Qx"),
        );
        close(
            p.qy.as_n_per_mm(),
            sign(sym.y_reversed()) * oqy.as_n_per_mm(),
            scale.q,
            &format!("{point} の Qy"),
        );
    }
    let divisions = 8;
    for e in Edge::ALL {
        // 辺に沿う向きと辺に直交する向き。x の辺は y に沿い、x に直交する
        let (along_reversed, normal_reversed) = match e {
            Edge::X0 | Edge::XLx => (sym.y_reversed(), sym.x_reversed()),
            Edge::Y0 | Edge::YLy => (sym.x_reversed(), sym.y_reversed()),
        };
        let mine = user.edge(e, divisions);
        let theirs = base.edge(sym.edge(e), divisions);
        for (i, a) in mine.iter().enumerate() {
            let b = theirs[if along_reversed { divisions - i } else { i }];
            let name = format!("{e} の {}", a.along.as_mm());
            close(
                a.moment.as_nmm_per_mm(),
                b.moment.as_nmm_per_mm(),
                scale.m,
                &format!("{name} の曲げモーメント"),
            );
            close(
                a.shear.as_n_per_mm(),
                sign(normal_reversed) * b.shear.as_n_per_mm(),
                scale.q,
                &format!("{name} のせん断力"),
            );
            close(
                a.reaction.as_n_per_mm(),
                b.reaction.as_n_per_mm(),
                scale.q,
                &format!("{name} の反力"),
            );
        }
        close(
            user.total_reaction(e).as_n(),
            base.total_reaction(sym.edge(e)).as_n(),
            0.0,
            &format!("{e} の辺の反力の合計"),
        );
    }
    for c in Corner::ALL {
        let (x, y) = match c {
            Corner::X0Y0 => (0.0, 0.0),
            Corner::XLxY0 => (lx, 0.0),
            Corner::XLxYLy => (lx, ly),
            Corner::X0YLy => (0.0, ly),
        };
        let (bx, by) = sym.point(x, y, lx, ly);
        close(
            user.corner_reaction(c).as_n(),
            base.corner_reaction(corner_at(bx, by)).as_n(),
            0.0,
            &format!("{c:?} の隅反力"),
        );
    }
    let mine = user.deflection_maximum();
    let theirs = base.deflection_maximum();
    let (x, y) = sym.point_back(theirs.x.as_mm(), theirs.y.as_mm(), base_lx, base_ly);
    close(mine.x.as_mm(), x, lx.max(ly), "たわみの最大の x");
    close(mine.y.as_mm(), y, lx.max(ly), "たわみの最大の y");
    close(mine.w.as_mm(), theirs.w.as_mm(), scale.w, "たわみの最大");
}

#[test]
fn every_combination_is_its_representative_mapped() {
    // 型に属する組をすべて（代表の向きの組を 8 つの対称で写したもの）入れる。代表の向きの組が自分自身に重なる
    // 型では、同じ組になる対称が 2 つ以上あり、多項式の場は重ならないので、どれで写すかで値が違う。入口は同じ
    // 組になる写し方のうち先のもの（恒等が最初、この並びの順）を選ぶので、ここでも先の対称で写す
    let (lx, ly) = (2000.0, 3000.0);
    let mut combinations = 0;
    for ty in SupportType::ALL.into_iter().filter(|t| t.is_stable()) {
        let representative = ty.representative();
        let base = solve_with_polynomial(lx, ly, representative);
        let mut seen = Vec::new();
        for sym in SYMMETRIES {
            let supports = sym.supports(representative);
            if seen.contains(&supports) {
                continue;
            }
            seen.push(supports);
            let (ulx, uly) = sym.lengths(lx, ly);
            let user = solve_with_polynomial(ulx, uly, supports);
            assert_eq!(user.support_type(), ty, "{}", supports.symbol());
            assert_maps(
                &user,
                &base,
                sym,
                &format!("{} の組 {}", ty.symbol(), supports.symbol()),
            );
            combinations += 1;
        }
    }
    assert_eq!(
        combinations, 76,
        "81 通りから不安定な 2 型の 5 つを除いた組"
    );
}
