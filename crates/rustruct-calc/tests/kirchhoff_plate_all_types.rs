//! 解ける 19 の支持条件の型すべてを、型の一覧（`SupportType::ALL` から不安定な 2 型を除いたもの）で回して
//! 確かめる。型ごとにテストを書き分けない。期待値は理論から言える値だけ（境界条件、釣り合い、対称、分割数を
//! 倍にしたときの変化、帯板と梁の極限、ν の換算）。各型は代表の向きの組（型の 4 文字がそのまま x = 0、y = 0、
//! x = Lx、y = Ly の辺）で解き、利用者の組への写しは対称のテストで確かめる。入口 `solve`（有限要素）からだけ見る

mod common;
use common::{
    Scale, assert_deviations_within_tolerance, assert_within, at, plate, reactions_minus_corners,
    rigidity, stable_types,
};
use rustruct_calc::kirchhoff_plate::{
    DIVISIONS, PointResponse, Solution, SupportType, solve, solve_with_divisions,
};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};

/// 有限要素の離散化の許容（ADR-0002 の照合の許容）。たわみと曲げモーメントはその量の最大の 0.5 %、
/// せん断力と辺の反力は 2 %
const W_M_TOLERANCE: f64 = 5e-3;
const Q_V_TOLERANCE: f64 = 2e-2;
/// 同じ連立を解いた値どうしの比較の許容（丸め）
const ROUND_OFF: f64 = 1e-9;
/// 対称な分割の上で、対称な 2 点の値どうしの比較の許容。節点の番号の振り方は対称でないので、連立の丸めが
/// 対称な 2 点で別々に効く。条件数の大きい自由辺のある板では 3 階の導関数（せん断力）に出て、分割数 40 で
/// 片持ちの板（FRRR）の自由辺どうしの隅の近くの Qy が板全体の最大の 1.2e-7（反復改良をしても変わらない）
const MIRRORED_MESH: f64 = 1e-6;

fn label(ty: SupportType, nu: f64) -> String {
    format!("{} ν = {nu}", ty.symbol())
}

/// 辺から板の内へ距離 `d` の点の座標。`along` は辺の始点（座標 0 の端）からの距離
fn inward(edge: Edge, along: f64, d: f64, lx: f64, ly: f64) -> (f64, f64) {
    match edge {
        Edge::X0 => (d, along),
        Edge::XLx => (lx - d, along),
        Edge::Y0 => (along, d),
        Edge::YLy => (along, ly - d),
    }
}

/// 辺に直交する方向の曲げモーメントとせん断力
fn normal_moment_and_shear(edge: Edge, r: &PointResponse) -> (f64, f64) {
    match edge {
        Edge::X0 | Edge::XLx => (r.mx.as_nmm_per_mm(), r.qx.as_n_per_mm()),
        Edge::Y0 | Edge::YLy => (r.my.as_nmm_per_mm(), r.qy.as_n_per_mm()),
    }
}

#[test]
fn every_edge_satisfies_its_boundary_conditions() {
    // 固定辺で w = 0・法線方向の傾き = 0、単純支持の辺で w = 0・M_n = 0、自由辺で M_n = 0・V_n = 0。
    // 各辺の両端から辺長の 5 % を除いた 41 点で（ADR-0008）。ν = 0 と 0.2
    let (lx, ly) = (2000.0, 3000.0);
    for ty in stable_types() {
        let supports = ty.representative();
        for nu in [0.0, 0.2] {
            let s = solve(&plate(lx, ly, nu, 10.0, supports)).unwrap();
            let scale = Scale::of(&s);
            for e in Edge::ALL {
                let (length, across) = match e {
                    Edge::X0 | Edge::XLx => (ly, lx),
                    Edge::Y0 | Edge::YLy => (lx, ly),
                };
                for i in 0..=40 {
                    let along = length * (0.05 + 0.9 * i as f64 / 40.0);
                    let r = |d: f64| {
                        let (x, y) = inward(e, along, d, lx, ly);
                        at(&s, x, y)
                    };
                    let what = format!("{} の {e} の {along}", label(ty, nu));
                    let edge = r(0.0);
                    let (moment, shear) = normal_moment_and_shear(e, &edge);
                    match supports.at(e) {
                        Support::Fixed | Support::SimplySupported => {
                            // 辺の節点は w と辺に沿う傾きを拘束するので、辺の上の w は形状関数の値として
                            // 厳密に 0。許容は丸め
                            assert_within(
                                edge.w.as_mm(),
                                0.0,
                                1e-12 * scale.w,
                                &format!("{what} の w"),
                            );
                        }
                        Support::Free => {}
                    }
                    match supports.at(e) {
                        Support::Fixed => {
                            // 法線方向の傾きも節点で拘束するので、有限要素の傾きは辺の上で厳密に 0。辺から
                            // h と 2h の点のたわみから傾きを (4 w(h) − w(2h)) / 2h で推す。推し方の誤差は
                            // h² の項で、h = 辺に直交する辺長の 1e-4 では傾き × 辺長 がたわみの最大の 1e-5 未満
                            let h = 1e-4 * across;
                            let slope = (4.0 * r(h).w.as_mm() - r(2.0 * h).w.as_mm()) / (2.0 * h);
                            assert_within(
                                slope * across,
                                0.0,
                                1e-5 * scale.w,
                                &format!("{what} の法線方向の傾き"),
                            );
                        }
                        Support::SimplySupported | Support::Free => {
                            // M_n = 0 は拘束でなく、ひずみエネルギーの最小で弱く満たす条件なので、残るのは
                            // 離散化の誤差。許容は照合と同じ 0.5 %
                            assert_within(
                                moment,
                                0.0,
                                W_M_TOLERANCE * scale.m,
                                &format!("{what} の M_n"),
                            );
                        }
                    }
                    if supports.at(e) == Support::Free {
                        // V_n = Q_n + ∂M_nt/∂t は、釣り合い Q_n = ∂M_n/∂n + ∂M_nt/∂t から
                        // V_n = 2 Q_n − ∂M_n/∂n（x の辺なら −D (w_xxx + (2 − ν) w_xyy) と同じ）。∂M_n/∂n は
                        // 辺から 0・h・2h の 3 点の片側差分（h = 辺に直交する辺長の 1e-3）。M_n と同じく弱く
                        // 満たす条件で、許容は照合と同じ 2 %
                        let h = 1e-3 * across;
                        let m = |d: f64| normal_moment_and_shear(e, &r(d)).0;
                        let inward_derivative =
                            (-3.0 * m(0.0) + 4.0 * m(h) - m(2.0 * h)) / (2.0 * h);
                        let derivative = match e {
                            Edge::X0 | Edge::Y0 => inward_derivative,
                            Edge::XLx | Edge::YLy => -inward_derivative,
                        };
                        assert_within(
                            2.0 * shear - derivative,
                            0.0,
                            Q_V_TOLERANCE * scale.q,
                            &format!("{what} の V_n"),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn edge_reactions_minus_corner_reactions_balance_the_total_load() {
    // 辺の反力の合計 − 隅反力の合計 = 荷重の合計（ADR-0004）。辺の反力の合計は節点の反力の和そのもので、
    // 丸め誤差（節点力は要素ごとに剛体の動きを引いて求め、連立は反復改良を 1 回するので、この大きさの板で
    // 4e-11 以下。サイト ADR-0009。許容 1e-7）で成り立つ。固定辺を含む隅と自由辺どうしの隅の
    // 隅反力は、隅の節点の w_xy を拘束するか（固定辺）、隅に支えが無いので（自由辺どうし）厳密に 0。
    // 自由辺の反力の合計も、拘束した節点が無いので厳密に 0
    for ty in stable_types() {
        let supports = ty.representative();
        for (lx, ly) in [(2000.0, 2000.0), (2000.0, 3000.0), (4000.0, 2000.0)] {
            let p = plate(lx, ly, 0.15, 8.0, supports);
            let s = solve(&p).unwrap();
            let what = format!("{} {lx}×{ly}", ty.symbol());
            let total = p.load.as_n_per_mm2() * lx * ly;
            assert_within(
                reactions_minus_corners(&s),
                total,
                1e-7 * total,
                &format!("{what} の釣り合い"),
            );
            for c in Corner::ALL {
                let (ex, ey) = c.edges();
                let (sx, sy) = (supports.at(ex), supports.at(ey));
                let both_free = sx == Support::Free && sy == Support::Free;
                if sx == Support::Fixed || sy == Support::Fixed || both_free {
                    assert_eq!(s.corner_reaction(c).as_n(), 0.0, "{what} の {c:?}");
                }
            }
            for e in Edge::ALL {
                if supports.at(e) == Support::Free {
                    assert_eq!(s.total_reaction(e).as_n(), 0.0, "{what} の {e}");
                }
            }
        }
    }
}

/// 長方形の 8 つの対称。利用者の板の座標を、先に x と y を入れ替え、次に x・y をそれぞれ裏返して比べる相手の
/// 板の座標にする。計算核の写し方（`Mapping`）とは別にテストの中で書き、結果の量の写しを独立に確かめる
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
    /// 利用者の辺 → 相手の板の辺
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

    /// 相手の板の組から、利用者の組へ
    fn supports(self, other: Supports) -> Supports {
        Supports {
            x0: other.at(self.edge(Edge::X0)),
            y0: other.at(self.edge(Edge::Y0)),
            x_lx: other.at(self.edge(Edge::XLx)),
            y_ly: other.at(self.edge(Edge::YLy)),
        }
    }

    /// 相手の板の辺長から利用者の板の辺長へ（逆も同じ）
    fn lengths(self, lx: f64, ly: f64) -> (f64, f64) {
        if self.transpose { (ly, lx) } else { (lx, ly) }
    }

    /// 利用者の点（利用者の板の辺長 lx・ly）→ 相手の板の点
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

    /// 利用者の x・y の向きが、相手の板で裏返るか。裏返る向きの微分を含むせん断力は符号が変わる
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

/// 利用者の板の解 `user` が、相手の板の解 `other` を対称 `sym` で写したものになっていること。点の応答、
/// 辺に沿った応答、隅反力。たわみは変わらず、入れ替えで Mx↔My・Qx↔Qy、裏返す向きのせん断力は符号が変わる
fn assert_maps(user: &Solution, other: &Solution, sym: Symmetry, tolerance: f64, what: &str) {
    let scale = Scale::of(other);
    let (lx, ly) = (user.plate().lx.as_mm(), user.plate().ly.as_mm());
    // 丸めは値そのものの大きさにも比例する。固定辺と自由辺の隅の値は板の最大より大きいので、大きいほうに対して
    let close = |a: f64, b: f64, unit: f64, name: &str| {
        assert_within(
            a,
            b,
            tolerance * unit.max(b.abs()),
            &format!("{what} の {name}"),
        );
    };
    for p in user.grid(8, 8) {
        let (x, y) = (p.x.as_mm(), p.y.as_mm());
        let (ox, oy) = sym.point(x, y, lx, ly);
        let o = at(other, ox, oy);
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
        let theirs = other.edge(sym.edge(e), divisions);
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
    }
    let corner_scale = Corner::ALL
        .iter()
        .map(|&c| other.corner_reaction(c).as_n().abs())
        .fold(0.0, f64::max);
    for c in Corner::ALL {
        let (x, y) = match c {
            Corner::X0Y0 => (0.0, 0.0),
            Corner::XLxY0 => (lx, 0.0),
            Corner::XLxYLy => (lx, ly),
            Corner::X0YLy => (0.0, ly),
        };
        let (ox, oy) = sym.point(x, y, lx, ly);
        close(
            user.corner_reaction(c).as_n(),
            other.corner_reaction(corner_at(ox, oy)).as_n(),
            corner_scale,
            &format!("{c:?} の隅反力"),
        );
    }
}

#[test]
fn every_combination_of_a_type_is_its_representative_mapped() {
    // 型に属する組をすべて（代表の向きの組を 8 つの対称で写したもの）入れ、代表の向きの板の解を写したものに
    // なること。どの組も計算核の中では同じ代表の向きの板を解くので、差は座標の換算の丸め
    let (lx, ly) = (2000.0, 3000.0);
    for ty in stable_types() {
        let representative = ty.representative();
        let base = solve(&plate(lx, ly, 0.2, 10.0, representative)).unwrap();
        let mut seen = Vec::new();
        for sym in SYMMETRIES {
            let supports = sym.supports(representative);
            if seen.contains(&supports) {
                continue;
            }
            seen.push(supports);
            let (ulx, uly) = sym.lengths(lx, ly);
            let user = solve(&plate(ulx, uly, 0.2, 10.0, supports)).unwrap();
            assert_eq!(user.support_type(), ty, "{}", supports.symbol());
            assert_maps(
                &user,
                &base,
                sym,
                ROUND_OFF,
                &format!("{} の組 {}", ty.symbol(), supports.symbol()),
            );
        }
    }
}

#[test]
fn a_symmetric_plate_follows_its_symmetry() {
    // 代表の向きの組がある対称で自分自身に重なる型では、板の解もその対称どおりになる。要素の分割も板の中央に
    // ついて対称なので、差は丸め。x と y を入れ替える対称は正方形の板で、ほかは 2000 × 3000 の板で見る
    for ty in stable_types() {
        let representative = ty.representative();
        for sym in SYMMETRIES {
            if sym.supports(representative) != representative {
                continue;
            }
            let (lx, ly) = if sym.transpose {
                (2000.0, 2000.0)
            } else {
                (2000.0, 3000.0)
            };
            let s = solve(&plate(lx, ly, 0.2, 10.0, representative)).unwrap();
            assert_maps(
                &s,
                &s,
                sym,
                MIRRORED_MESH,
                &format!("{} の {sym:?}", ty.symbol()),
            );
        }
    }
}

/// ADR-0006 の決め方。ν = 0.2 で、短い辺の分割数を DIVISIONS から倍にしたときの差が、参照解との照合と同じ
/// 許容（隅から辺長の 5 % を除いて、w・M は 0.5 %、Q・V は 2 %）に収まる。比は代表の向きの Ly/Lx。
/// 19 型を `parts` 組に分けた `part` 番目の組（型の一覧の位置で `parts` おき）だけを見る
fn assert_doubling_the_divisions_changes_nothing(ratio: f64, part: usize, parts: usize) {
    let lx = 2000.0;
    for ty in stable_types().skip(part).step_by(parts) {
        let p = plate(lx, ratio * lx, 0.2, 10.0, ty.representative());
        let coarse = solve_with_divisions(&p, DIVISIONS).unwrap();
        let fine = solve_with_divisions(&p, 2 * DIVISIONS).unwrap();
        assert_deviations_within_tolerance(&fine, &coarse, &format!("{} 比 {ratio}", ty.symbol()));
    }
}

// 1 つの型で粗い解と倍の分割の解を求めると、縦横比 1 で約 0.8 s、4 で約 2.4 s かかる（rustruct-calc は
// opt-level 2）。
// 縦横比と型の組でテストを分け、並行に走らせる。組の数は 1 つのテストが数秒に収まるように選んだ
macro_rules! doubling_tests {
    ($($name:ident: $ratio:expr, $part:expr, $parts:expr;)+) => {
        $(
            #[test]
            fn $name() {
                assert_doubling_the_divisions_changes_nothing($ratio, $part, $parts);
            }
        )+
    };
}

doubling_tests! {
    doubling_the_divisions_changes_nothing_at_aspect_ratio_1_part_0: 1.0, 0, 2;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_1_part_1: 1.0, 1, 2;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_2_part_0: 2.0, 0, 4;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_2_part_1: 2.0, 1, 4;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_2_part_2: 2.0, 2, 4;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_2_part_3: 2.0, 3, 4;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_4_part_0: 4.0, 0, 6;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_4_part_1: 4.0, 1, 6;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_4_part_2: 4.0, 2, 6;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_4_part_3: 4.0, 3, 6;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_4_part_4: 4.0, 4, 6;
    doubling_the_divisions_changes_nothing_at_aspect_ratio_4_part_5: 4.0, 5, 6;
}

/// 長さ `l` の梁に等分布荷重 `q`、端 a（x = 0）と端 b（x = l）の支持条件。x の点の (D·w, M, Q)。M は下側引張、
/// Q = dM/dx。梁が不安定な組（単純支持と自由、自由どうし）は `None`
fn beam(a: Support, b: Support, l: f64, q: f64, x: f64) -> Option<(f64, f64, f64)> {
    use Support::{Fixed, Free, SimplySupported};
    let from_a = |a: Support, b: Support, x: f64| -> Option<(f64, f64, f64)> {
        Some(match (a, b) {
            (Fixed, Fixed) => (
                q * x * x * (l - x) * (l - x) / 24.0,
                q * (6.0 * l * x - 6.0 * x * x - l * l) / 12.0,
                q * (l / 2.0 - x),
            ),
            (SimplySupported, SimplySupported) => (
                q * x * (l.powi(3) - 2.0 * l * x * x + x.powi(3)) / 24.0,
                q * x * (l - x) / 2.0,
                q * (l / 2.0 - x),
            ),
            (Fixed, SimplySupported) => (
                q * x * x * (3.0 * l * l - 5.0 * l * x + 2.0 * x * x) / 48.0,
                -q * (l * l - 5.0 * l * x + 4.0 * x * x) / 8.0,
                q * (5.0 * l - 8.0 * x) / 8.0,
            ),
            (Fixed, Free) => (
                q * x * x * (6.0 * l * l - 4.0 * l * x + x * x) / 24.0,
                -q * (l - x) * (l - x) / 2.0,
                q * (l - x),
            ),
            _ => return None,
        })
    };
    // 逆向きの組は梁を裏返したもの。x を l − x にし、Q の符号を変える
    from_a(a, b, x).or_else(|| from_a(b, a, l - x).map(|(dw, m, v)| (dw, m, -v)))
}

/// 帯板の向き。`AcrossX` は x = 0 と x = Lx の辺の間に架かる帯（y に長い板）
#[derive(Debug, Clone, Copy)]
enum Strip {
    AcrossX,
    AcrossY,
}

/// 帯の両端の支持条件と、帯に沿う向き・直交する向きの応答の取り出し
impl Strip {
    fn ends(self, s: Supports) -> (Support, Support) {
        match self {
            Strip::AcrossX => (s.x0, s.x_lx),
            Strip::AcrossY => (s.y0, s.y_ly),
        }
    }
    fn end_edges(self) -> (Edge, Edge) {
        match self {
            Strip::AcrossX => (Edge::X0, Edge::XLx),
            Strip::AcrossY => (Edge::Y0, Edge::YLy),
        }
    }
    /// 帯の中の点 (帯に沿う座標 u, 帯に直交する座標 v)
    fn point(self, u: f64, v: f64) -> (f64, f64) {
        match self {
            Strip::AcrossX => (u, v),
            Strip::AcrossY => (v, u),
        }
    }
    /// 帯の向きの曲げモーメント、直交する向きの曲げモーメント、帯の向きのせん断力
    fn moments_and_shear(self, r: &PointResponse) -> (f64, f64, f64) {
        match self {
            Strip::AcrossX => (
                r.mx.as_nmm_per_mm(),
                r.my.as_nmm_per_mm(),
                r.qx.as_n_per_mm(),
            ),
            Strip::AcrossY => (
                r.my.as_nmm_per_mm(),
                r.mx.as_nmm_per_mm(),
                r.qy.as_n_per_mm(),
            ),
        }
    }
}

/// 帯の向きの断面 v で、板の解が帯板（梁の解を板の曲げ剛性 D で。直交する向きの曲げモーメントは ν·M）に
/// 一致すること。帯の 11 点のたわみ・曲げモーメント・せん断力と、帯の端の辺の反力
fn assert_strip(s: &Solution, strip: Strip, span: f64, v: f64, what: &str) {
    let p = s.plate();
    let (q, d, nu) = (p.load.as_n_per_mm2(), rigidity(p), p.poisson_ratio.value());
    let (a, b) = strip.ends(p.supports);
    let beam_at = |u: f64| beam(a, b, span, q, u).expect("帯の両端で支える組");
    let samples: Vec<(f64, (f64, f64, f64))> = (0..=10)
        .map(|i| span * i as f64 / 10.0)
        .map(|u| (u, beam_at(u)))
        .collect();
    let max = |f: fn(&(f64, f64, f64)) -> f64| {
        samples.iter().map(|(_, r)| f(r).abs()).fold(0.0, f64::max)
    };
    let (w_max, m_max, q_max) = (max(|r| r.0) / d, max(|r| r.1), max(|r| r.2));
    for &(u, (dw, m, shear)) in &samples {
        let (x, y) = strip.point(u, v);
        let r = at(s, x, y);
        let (m_along, m_across, q_along) = strip.moments_and_shear(&r);
        let here = format!("{what} の ({x}, {y})");
        assert_within(
            r.w.as_mm(),
            dw / d,
            W_M_TOLERANCE * w_max,
            &format!("{here} の w"),
        );
        assert_within(
            m_along,
            m,
            W_M_TOLERANCE * m_max,
            &format!("{here} の帯の向きの M"),
        );
        assert_within(
            m_across,
            nu * m,
            W_M_TOLERANCE * m_max,
            &format!("{here} の直交する向きの M"),
        );
        assert_within(
            q_along,
            shear,
            Q_V_TOLERANCE * q_max,
            &format!("{here} の Q"),
        );
    }
    // 帯の端の辺の反力は、ねじれの無い断面ではせん断力と同じ。座標 0 の端で Q、反対の端で −Q（支える向き）
    let (edge_a, edge_b) = strip.end_edges();
    let length_along_edge = match strip {
        Strip::AcrossX => p.ly.as_mm(),
        Strip::AcrossY => p.lx.as_mm(),
    };
    for (edge, support, expected) in [(edge_a, a, beam_at(0.0).2), (edge_b, b, -beam_at(span).2)] {
        if support == Support::Free {
            continue;
        }
        let samples = s.edge(edge, 1000);
        let i = (v / length_along_edge * 1000.0).round() as usize;
        assert_within(
            samples[i].reaction.as_n_per_mm(),
            expected,
            Q_V_TOLERANCE * q_max,
            &format!("{what} の {edge} の反力"),
        );
    }
}

#[test]
fn a_long_plate_approaches_the_strip_across_its_short_span() {
    // 長い板の長い向きの中央の断面は、短い向きに架かる帯板（梁の解を板の曲げ剛性で）になる。ν = 0.2 なら
    // 直交する向きの曲げモーメントは ν·M。長い板は短い辺長の 12 倍で、両端の辺の乱れは中央まで短い辺長ごとに
    // 指数的に減る。減り方がいちばん遅いのは固定と自由が向かい合う帯（片持ち）の板で、12 倍で中央に最大の
    // 6e-4 程度が残る（ほかの帯は 1e-5 未満）。許容は照合と同じ（w・M は 0.5 %、Q・V は 2 %）で、この残りを含む。
    //
    // 帯板の極限が当てはまるのは、帯の両端（向かい合う 2 辺）が梁として支えられる組（固定どうし、単純支持
    // どうし、固定と単純支持、固定と自由）のときだけ。単純支持と自由・自由どうしが向かい合う向きには帯が
    // 架からない（梁が不安定）ので見ない。両方の向きがそうなる RRSS（向かい合う 2 組がどちらも単純支持と
    // 自由）には、帯板の極限が無い
    let span = 2000.0;
    let length = 12.0 * span;
    let mut checked = Vec::new();
    for ty in stable_types() {
        let supports = ty.representative();
        for strip in [Strip::AcrossX, Strip::AcrossY] {
            let (a, b) = strip.ends(supports);
            if beam(a, b, 1.0, 1.0, 0.5).is_none() {
                continue;
            }
            let (lx, ly) = strip.point(span, length);
            let s = solve(&plate(lx, ly, 0.2, 10.0, supports)).unwrap();
            assert_strip(
                &s,
                strip,
                span,
                length / 2.0,
                &format!("{} の {strip:?}", ty.symbol()),
            );
            checked.push(ty);
        }
    }
    let without: Vec<SupportType> = stable_types().filter(|t| !checked.contains(t)).collect();
    assert_eq!(without, [SupportType::RRSS], "帯板の極限の無い型");
}

#[test]
fn a_one_way_plate_with_zero_poisson_ratio_is_a_beam() {
    // 向かい合う 2 辺が自由で、残る 2 辺が梁として支える組（FRFR・FRRR・FRSR・RSRS）は、ν = 0 なら板全体が
    // 梁の解に一致する（梁の解は自由辺の M_n = −D (w_nn + ν w_tt) = 0 と V_n = 0 をそのまま満たす）。長さに
    // 依らないので、自由辺の長さを梁の長さの 1/4・1・4 倍にして、隅まで含めた 9 × 9 のグリッドと支える辺の
    // 反力で見る。許容は照合と同じ（w・M は 0.5 %、Q・V は 2 %）
    let span = 2000.0;
    let mut checked = Vec::new();
    for ty in stable_types() {
        let supports = ty.representative();
        let (strip, free) = match (supports.x0, supports.y0, supports.x_lx, supports.y_ly) {
            (_, Support::Free, _, Support::Free) => (Strip::AcrossX, true),
            (Support::Free, _, Support::Free, _) => (Strip::AcrossY, true),
            _ => (Strip::AcrossX, false),
        };
        if !free {
            continue;
        }
        checked.push(ty);
        for width in [0.25 * span, span, 4.0 * span] {
            let (lx, ly) = strip.point(span, width);
            let s = solve(&plate(lx, ly, 0.0, 10.0, supports)).unwrap();
            for j in 0..=8 {
                assert_strip(
                    &s,
                    strip,
                    span,
                    width * j as f64 / 8.0,
                    &format!("{} 幅 {width}", ty.symbol()),
                );
            }
        }
    }
    assert_eq!(
        checked,
        [
            SupportType::FRFR,
            SupportType::FRRR,
            SupportType::FRSR,
            SupportType::RSRS
        ]
    );
}

#[test]
fn poisson_ratio_scales_the_deflection_and_mixes_the_moments_without_free_edges() {
    // 自由辺の無い型では無次元の解が ν に依らない（ADR-0005）: w ∝ 1/D ∝ 1 − ν²、Mx(ν) = Mx(0) + ν·My(0)、
    // せん断力は ν に依らず、隅反力 2 D (1 − ν) |w_xy| は (1 − ν) 倍。有限要素でも同じ。ひずみエネルギーの ν の
    // 項 2ν (w_xx w_yy − w_xy²) は、要素の間で w と傾きが連続なので板全体では辺の上の値だけで決まり、固定辺と
    // 単純支持の辺（辺の上で w が 0）では 0 になる。差は連立の丸め（許容 1e-9）
    let (lx, ly) = (2000.0, 2800.0);
    let nu = 0.3;
    let mut checked = Vec::new();
    for ty in stable_types() {
        let supports = ty.representative();
        if Edge::ALL.iter().any(|&e| supports.at(e) == Support::Free) {
            continue;
        }
        checked.push(ty);
        let s0 = solve(&plate(lx, ly, 0.0, 10.0, supports)).unwrap();
        let s1 = solve(&plate(lx, ly, nu, 10.0, supports)).unwrap();
        let scale = Scale::of(&s0);
        let what = ty.symbol();
        for (p0, p1) in s0.grid(10, 10).iter().zip(s1.grid(10, 10)) {
            let here = format!("{what} の ({}, {})", p0.x.as_mm(), p0.y.as_mm());
            assert_within(
                p1.w.as_mm(),
                (1.0 - nu * nu) * p0.w.as_mm(),
                ROUND_OFF * scale.w,
                &format!("{here}: w ∝ 1 − ν²"),
            );
            assert_within(
                p1.mx.as_nmm_per_mm(),
                p0.mx.as_nmm_per_mm() + nu * p0.my.as_nmm_per_mm(),
                ROUND_OFF * scale.m,
                &format!("{here}: Mx(ν) = Mx(0) + ν·My(0)"),
            );
            assert_within(
                p1.my.as_nmm_per_mm(),
                p0.my.as_nmm_per_mm() + nu * p0.mx.as_nmm_per_mm(),
                ROUND_OFF * scale.m,
                &format!("{here}: My(ν) = My(0) + ν·Mx(0)"),
            );
            assert_within(
                p1.qx.as_n_per_mm(),
                p0.qx.as_n_per_mm(),
                ROUND_OFF * scale.q,
                &format!("{here}: Qx は ν に依らない"),
            );
            assert_within(
                p1.qy.as_n_per_mm(),
                p0.qy.as_n_per_mm(),
                ROUND_OFF * scale.q,
                &format!("{here}: Qy は ν に依らない"),
            );
        }
        for c in Corner::ALL {
            let r0 = s0.corner_reaction(c).as_n();
            assert_within(
                s1.corner_reaction(c).as_n(),
                (1.0 - nu) * r0,
                ROUND_OFF * r0.abs(),
                &format!("{what} の {c:?} の隅反力"),
            );
        }
    }
    assert_eq!(
        checked,
        [
            SupportType::FFFF,
            SupportType::FFFS,
            SupportType::FFSS,
            SupportType::FSFS,
            SupportType::FSSS,
            SupportType::SSSS
        ]
    );
}
