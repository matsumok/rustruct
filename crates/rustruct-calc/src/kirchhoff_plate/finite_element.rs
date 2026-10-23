//! Kirchhoff 板を長方形の共形要素で解く有限要素の解き方。板の中央を原点に、x の半辺長を 1、y の半辺長を
//! b = Ly/Lx とした無次元の板（p = 1、D = 1）を、支持条件の型によらず一本のコードで解く。
//!
//! 要素は節点に 4 つの自由度（w、w_x、w_y、w_xy）を持つ双 3 次エルミートの長方形で、要素の間で w と
//! その 1 階の導関数が連続（共形）。要素の剛性は曲げのひずみエネルギーから ν を含めて組み、等分布荷重は
//! 形状関数で節点に配る。形状関数が x の多項式と y の多項式の積なので、剛性も荷重も 1 次元の積分の積で
//! 閉じる。1 次元の積分は 4 点の Gauss 積分で厳密。
//!
//! 節点は辺に寄せて密に置く（余弦の間隔）。せん断力と辺の反力は隅の近くで急に変わり、辺の曲げモーメントは
//! 辺の上で読むので、等間隔より辺の近くを細かくするほうが同じ節点数で精度が出る。
//!
//! 支持条件は節点の自由度の拘束だけで表す。固定辺はその辺の上で w と法線方向の傾きが 0 なので、辺に沿う
//! 導関数も 0 になり 4 つすべてを拘束する。単純支持の辺は w が 0 なので w と辺に沿う傾きを拘束し、法線方向の
//! 傾きとそのねじれ（w_xy）は残す。自由辺は何も拘束しない。隅の節点は会う 2 辺の拘束の和。
//!
//! 辺の反力は拘束した w の自由度の釣り合いの力（節点の反力）から作る。節点の反力を受け持ち区間に載る分布の
//! 積分と見て、節点の間で線形な分布に直す。隅の節点は隅反力（隅の w_xy から求める）の分を足し戻してから
//! 会う辺に振り分ける。こうすると辺の反力の積分から隅反力を引いたものが荷重の合計に丸め誤差で一致する。
//! たわみの導関数は形状関数から解析的に取る。w と w_xy は要素の間で連続なのでそのまま。要素の間で跳ぶ
//! 2 階・3 階の導関数は要素の中央の値（2 階は補間の誤差を差分で補正したもの）を多項式で結び、辺の上へは
//! 端の要素の中央から外挿する。

use std::f64::consts::PI;

use rustruct_types::kirchhoff_plate::{Corner, Edge, Support, Supports};

use super::band::BandMatrix;
use super::levy::Derivatives;
use super::solver::Solver;

/// 長い辺の分割数の上限。縦横比がこれを超える板は中央の要素が大きくなるが、連立の大きさを抑える
const MAX_DIVISIONS: usize = 512;

/// 短い辺を `divisions` 分割したときの節点の座標 (x の節点, y の節点)。短い辺は両端に寄せた余弦の間隔
/// （中央について対称、両端が密）。長い辺は、両端に短い辺と同じ間隔の並びを半分ずつ置き、その間を短い辺の
/// 中央の要素と同じ大きさで等間隔に埋める。辺の近くの要素の大きさが辺長によらず揃い、細長い要素ができない
fn node_positions(b: f64, divisions: usize) -> (Vec<f64>, Vec<f64>) {
    let (short, long) = (b.min(1.0), b.max(1.0));
    let cosine: Vec<f64> = (0..=divisions)
        .map(|k| -short * (PI * k as f64 / divisions as f64).cos())
        .collect();
    let longer = if long == short {
        cosine.clone()
    } else {
        let half = divisions / 2;
        let offset = long - short;
        let middle = 2.0 * offset;
        let step = cosine[half + 1] - cosine[half];
        let count = ((middle / step).ceil() as usize).clamp(2, MAX_DIVISIONS - divisions);
        let count = count + count % 2;
        let mut xs: Vec<f64> = cosine[..=half].iter().map(|x| x - offset).collect();
        xs.extend((1..count).map(|k| -offset + middle * k as f64 / count as f64));
        xs.extend(cosine[half..].iter().map(|x| x + offset));
        xs
    };
    if b >= 1.0 {
        (cosine, longer)
    } else {
        (longer, cosine)
    }
}

/// 短い辺を `divisions` 分割したときの (x の分割数, y の分割数)。収束のテストのためにある
pub fn mesh_divisions(b: f64, divisions: usize) -> (usize, usize) {
    let (xs, ys) = node_positions(b, divisions);
    (xs.len() - 1, ys.len() - 1)
}

/// 1 次元のエルミート多項式。半長 h の区間の ξ ∈ [−1, 1] での 4 つの基底（ξ = −1 の値と傾き、ξ = 1 の値と傾き）
/// について、値と x についての 1〜3 階の導関数を `[基底][階数]` で返す
fn hermite(h: f64, xi: f64) -> [[f64; 4]; 4] {
    let xi2 = xi * xi;
    let xi3 = xi2 * xi;
    [
        [
            (2.0 - 3.0 * xi + xi3) / 4.0,
            (3.0 * xi2 - 3.0) / (4.0 * h),
            1.5 * xi / (h * h),
            1.5 / (h * h * h),
        ],
        [
            h * (1.0 - xi - xi2 + xi3) / 4.0,
            (-1.0 - 2.0 * xi + 3.0 * xi2) / 4.0,
            (3.0 * xi - 1.0) / (2.0 * h),
            1.5 / (h * h),
        ],
        [
            (2.0 + 3.0 * xi - xi3) / 4.0,
            (3.0 - 3.0 * xi2) / (4.0 * h),
            -1.5 * xi / (h * h),
            -1.5 / (h * h * h),
        ],
        [
            h * (-1.0 - xi + xi2 + xi3) / 4.0,
            (-1.0 + 2.0 * xi + 3.0 * xi2) / 4.0,
            (3.0 * xi + 1.0) / (2.0 * h),
            1.5 / (h * h),
        ],
    ]
}

/// 要素の自由度の数。並びは局所の節点 (a, b)（ξ = 2a − 1、η = 2b − 1）の順に、節点ごとに w、w_x、w_y、w_xy
const ELEMENT_DOFS: usize = 16;

/// 要素の自由度 e を、x の基底の番号、y の基底の番号、x の傾きの自由度か、y の傾きの自由度かに分ける
fn split(e: usize) -> (usize, usize, usize, usize) {
    let (node, k) = (e / 4, e % 4);
    let (sx, sy) = (k & 1, k >> 1);
    (2 * (node % 2) + sx, 2 * (node / 2) + sy, sx, sy)
}

/// 4 点の Gauss 積分の点と重み（[−1, 1]）
const GAUSS: [(f64, f64); 4] = [
    (-0.861_136_311_594_052_6, 0.347_854_845_137_453_9),
    (-0.339_981_043_584_856_3, 0.652_145_154_862_546_1),
    (0.339_981_043_584_856_3, 0.652_145_154_862_546_1),
    (0.861_136_311_594_052_6, 0.347_854_845_137_453_9),
];

/// 半長 1 の区間でのエルミート基底の 1 次元の積分。要素の剛性と荷重はこれの積で組む
struct Integrals {
    /// same[k][m][m'] = ∫ φ_m^(k) φ_m'^(k) dξ（k = 0, 1, 2）
    same: [[[f64; 4]; 4]; 3],
    /// cross[m][m'] = ∫ φ_m'' φ_m' dξ
    cross: [[f64; 4]; 4],
    /// load[m] = ∫ φ_m dξ
    load: [f64; 4],
}

impl Integrals {
    fn new() -> Self {
        let mut g = Integrals {
            same: [[[0.0; 4]; 4]; 3],
            cross: [[0.0; 4]; 4],
            load: [0.0; 4],
        };
        for (xi, weight) in GAUSS {
            let f = hermite(1.0, xi);
            for m in 0..4 {
                g.load[m] += weight * f[m][0];
                for n in 0..4 {
                    for (k, same) in g.same.iter_mut().enumerate() {
                        same[m][n] += weight * f[m][k] * f[n][k];
                    }
                    g.cross[m][n] += weight * f[m][2] * f[n][0];
                }
            }
        }
        g
    }

    /// 半長 (hx, hy) の要素の剛性行列と、等分布荷重 1 の節点荷重。ひずみエネルギーは
    /// ½ ∫ (w_xx² + w_yy² + 2ν w_xx w_yy + 2(1 − ν) w_xy²) dA。傾きの自由度の基底は半長に比例し、
    /// 導関数は階数ごとに半長で割れる
    fn element(
        &self,
        hx: f64,
        hy: f64,
        nu: f64,
    ) -> ([[f64; ELEMENT_DOFS]; ELEMENT_DOFS], [f64; ELEMENT_DOFS]) {
        let mut k = [[0.0; ELEMENT_DOFS]; ELEMENT_DOFS];
        let mut load = [0.0; ELEMENT_DOFS];
        let area = hx * hy;
        for (r, row) in k.iter_mut().enumerate() {
            let (mr, nr, sr, tr) = split(r);
            load[r] =
                area * hx.powi(sr as i32) * hy.powi(tr as i32) * self.load[mr] * self.load[nr];
            for (c, value) in row.iter_mut().enumerate() {
                let (mc, nc, sc, tc) = split(c);
                let scale = area * hx.powi((sr + sc) as i32) * hy.powi((tr + tc) as i32);
                let xx = self.same[2][mr][mc] * self.same[0][nr][nc] / hx.powi(4);
                let yy = self.same[0][mr][mc] * self.same[2][nr][nc] / hy.powi(4);
                let mixed = (nu
                    * (self.cross[mr][mc] * self.cross[nc][nr]
                        + self.cross[mc][mr] * self.cross[nr][nc])
                    + 2.0 * (1.0 - nu) * self.same[1][mr][mc] * self.same[1][nr][nc])
                    / (hx * hx * hy * hy);
                *value = scale * (xx + yy + mixed);
            }
        }
        (k, load)
    }
}

/// 辺の上の節点で拘束する自由度（w、w_x、w_y、w_xy の順）。`normal_x` はその辺の法線が x 方向か
fn constrained(support: Support, normal_x: bool) -> [bool; 4] {
    match support {
        Support::Fixed => [true; 4],
        Support::SimplySupported if normal_x => [true, false, true, false],
        Support::SimplySupported => [true, true, false, false],
        Support::Free => [false; 4],
    }
}

/// 節点 `positions` の間で線形な分布で、節点 k の受け持ち区間（両隣の節点との中点の間。端は端の節点から
/// 中点まで）の積分が `cells[k]` に等しいものの節点の値。受け持ち区間は要素の半分ずつからなり、要素の半分の
/// 積分は線形な分布の両端の値の 1 : 3 の重み。三重対角の連立を前進消去と後退代入で解く
fn piecewise_linear_from_cells(positions: &[f64], cells: &[f64]) -> Vec<f64> {
    let n = positions.len();
    let h = |k: usize| positions[k + 1] - positions[k];
    let (mut lower, mut diag, mut upper) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    for k in 0..n {
        if k > 0 {
            lower[k] = h(k - 1) / 8.0;
            diag[k] += 3.0 * h(k - 1) / 8.0;
        }
        if k + 1 < n {
            upper[k] = h(k) / 8.0;
            diag[k] += 3.0 * h(k) / 8.0;
        }
    }
    let mut rhs = cells.to_vec();
    for k in 1..n {
        let m = lower[k] / diag[k - 1];
        diag[k] -= m * upper[k - 1];
        rhs[k] -= m * rhs[k - 1];
    }
    let mut v = vec![0.0; n];
    v[n - 1] = rhs[n - 1] / diag[n - 1];
    for k in (0..n - 1).rev() {
        v[k] = (rhs[k] - upper[k] * v[k + 1]) / diag[k];
    }
    v
}

pub struct FiniteElement {
    nu: f64,
    supports: Supports,
    /// 節点の x 座標（nx + 1 個）と y 座標（ny + 1 個）。両端が辺
    xs: Vec<f64>,
    ys: Vec<f64>,
    /// 節点の自由度の値。節点ごとに w、w_x、w_y、w_xy。節点の番号は `node`
    u: Vec<f64>,
    /// 辺ごとの、辺の始点（座標の小さい端）から節点の順の辺の反力（支える向きが正）。自由辺は空。
    /// 並びは `Edge::ALL`
    edge_reactions: [Vec<f64>; 4],
}

/// 解く。短い辺を `divisions` 分割する。連立が解けなければ `None`
pub fn solve(b: f64, nu: f64, supports: Supports, divisions: usize) -> Option<FiniteElement> {
    assert!(
        divisions >= 4 && divisions.is_multiple_of(2),
        "分割数は 4 以上の偶数"
    );
    let (xs, ys) = node_positions(b, divisions);
    let (nx, ny) = (xs.len() - 1, ys.len() - 1);
    let mut fe = FiniteElement {
        nu,
        supports,
        xs,
        ys,
        u: vec![0.0; 4 * (nx + 1) * (ny + 1)],
        edge_reactions: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
    };
    let integrals = Integrals::new();
    let fixed = fe.constrained_dofs();
    // 拘束しない自由度だけに番号を振り直す
    let mut reduced = vec![None; fixed.len()];
    let mut free = 0;
    for (i, &is_fixed) in fixed.iter().enumerate() {
        if !is_fixed {
            reduced[i] = Some(free);
            free += 1;
        }
    }
    let elements: Vec<(usize, usize)> =
        (0..ny).flat_map(|j| (0..nx).map(move |i| (i, j))).collect();
    let bandwidth = elements
        .iter()
        .map(|&(i, j)| {
            let r: Vec<usize> = fe
                .element_dofs(i, j)
                .iter()
                .filter_map(|&d| reduced[d])
                .collect();
            match (r.iter().min(), r.iter().max()) {
                (Some(lo), Some(hi)) => hi - lo,
                _ => 0,
            }
        })
        .max()
        .unwrap_or(0);
    let mut k = BandMatrix::zeros(free, bandwidth);
    let mut f = vec![0.0; free];
    for &(i, j) in &elements {
        let (ke, fe_load) = integrals.element(fe.hx(i), fe.hy(j), nu);
        let dofs = fe.element_dofs(i, j);
        for (r, &dr) in dofs.iter().enumerate() {
            let Some(gr) = reduced[dr] else { continue };
            f[gr] += fe_load[r];
            for (c, &dc) in dofs.iter().enumerate() {
                if let Some(gc) = reduced[dc]
                    && gr >= gc
                {
                    k.add(gr, gc, ke[r][c]);
                }
            }
        }
    }
    let x = k.solve(f)?;
    for (i, &r) in reduced.iter().enumerate() {
        if let Some(r) = r {
            fe.u[i] = x[r];
        }
    }
    // 拘束した自由度の反力（支える向き、つまり荷重と逆向きが正）: 節点荷重 − 剛性 × 変位
    let mut reaction = vec![0.0; fixed.len()];
    for &(i, j) in &elements {
        let dofs = fe.element_dofs(i, j);
        if !dofs.iter().any(|&d| fixed[d]) {
            continue;
        }
        let (ke, fe_load) = integrals.element(fe.hx(i), fe.hy(j), nu);
        for (r, &dr) in dofs.iter().enumerate() {
            if !fixed[dr] {
                continue;
            }
            let ku: f64 = dofs
                .iter()
                .enumerate()
                .map(|(c, &dc)| ke[r][c] * fe.u[dc])
                .sum();
            reaction[dr] += fe_load[r] - ku;
        }
    }
    fe.distribute_reactions(&reaction);
    Some(fe)
}

impl FiniteElement {
    fn nx(&self) -> usize {
        self.xs.len() - 1
    }
    fn ny(&self) -> usize {
        self.ys.len() - 1
    }
    fn b(&self) -> f64 {
        self.ys[self.ny()]
    }
    /// 要素の列 i の x の半長
    fn hx(&self, i: usize) -> f64 {
        (self.xs[i + 1] - self.xs[i]) / 2.0
    }
    fn hy(&self, j: usize) -> f64 {
        (self.ys[j + 1] - self.ys[j]) / 2.0
    }

    /// 節点 (i, j) の番号。節点の数が少ない向きを先に回して帯の幅を抑える
    fn node(&self, i: usize, j: usize) -> usize {
        if self.nx() <= self.ny() {
            j * (self.nx() + 1) + i
        } else {
            i * (self.ny() + 1) + j
        }
    }

    /// 要素 (i, j) の 16 の自由度の全体番号。並びは `ELEMENT_DOFS` のとおり
    fn element_dofs(&self, i: usize, j: usize) -> [usize; ELEMENT_DOFS] {
        let mut dofs = [0; ELEMENT_DOFS];
        for (e, d) in dofs.iter_mut().enumerate() {
            let (node, k) = (e / 4, e % 4);
            *d = 4 * self.node(i + node % 2, j + node / 2) + k;
        }
        dofs
    }

    /// 全自由度の拘束の有無。辺の上の節点はその辺の拘束を、隅の節点は会う 2 辺の拘束の和を受ける
    fn constrained_dofs(&self) -> Vec<bool> {
        let mut fixed = vec![false; self.u.len()];
        for j in 0..=self.ny() {
            for i in 0..=self.nx() {
                let mut on = [false; 4];
                let edges = [
                    (i == 0, Edge::X0),
                    (i == self.nx(), Edge::XLx),
                    (j == 0, Edge::Y0),
                    (j == self.ny(), Edge::YLy),
                ];
                for (here, edge) in edges {
                    if !here {
                        continue;
                    }
                    let normal_x = matches!(edge, Edge::X0 | Edge::XLx);
                    let c = constrained(self.supports.at(edge), normal_x);
                    for (o, c) in on.iter_mut().zip(c) {
                        *o |= c;
                    }
                }
                let node = self.node(i, j);
                for (k, &c) in on.iter().enumerate() {
                    fixed[4 * node + k] = c;
                }
            }
        }
        fixed
    }

    /// 辺の上の節点を始点（座標の小さい端）から順に
    fn edge_nodes(&self, edge: Edge) -> Vec<usize> {
        match edge {
            Edge::X0 => (0..=self.ny()).map(|j| self.node(0, j)).collect(),
            Edge::XLx => (0..=self.ny()).map(|j| self.node(self.nx(), j)).collect(),
            Edge::Y0 => (0..=self.nx()).map(|i| self.node(i, 0)).collect(),
            Edge::YLy => (0..=self.nx()).map(|i| self.node(i, self.ny())).collect(),
        }
    }

    /// 辺に沿う節点の座標
    fn edge_positions(&self, edge: Edge) -> &[f64] {
        match edge {
            Edge::X0 | Edge::XLx => &self.ys,
            Edge::Y0 | Edge::YLy => &self.xs,
        }
    }

    /// 隅の節点の (i, j)
    fn corner_node(&self, corner: Corner) -> (usize, usize) {
        match corner {
            Corner::X0Y0 => (0, 0),
            Corner::XLxY0 => (self.nx(), 0),
            Corner::XLxYLy => (self.nx(), self.ny()),
            Corner::X0YLy => (0, self.ny()),
        }
    }

    /// 隅の節点の w_xy から隅反力（板を下に押さえる向きが正）。`corner_w_xy` と同じ値を入口が同じ式で
    /// 有次元にするので、ここの符号の向きと入口の向きは揃っていなければならない
    fn corner_force(&self, corner: Corner) -> f64 {
        let orientation = match corner {
            Corner::X0Y0 | Corner::XLxYLy => 1.0,
            Corner::XLxY0 | Corner::X0YLy => -1.0,
        };
        2.0 * (1.0 - self.nu) * orientation * self.corner_w_xy(corner)
    }

    /// 節点の反力を辺の反力の分布にする。辺の内部の節点の反力は、両隣の要素の半分ずつの受け持ち区間に
    /// 載る反力の積分。隅の節点の反力は、会う 2 辺の端の要素の半分ずつの受け持ちから隅反力を引いたものなので、
    /// 隅反力を足し戻してから支持のある辺に振り分ける（両方に支持があれば隣の節点の受け持ちあたりの値の比で、
    /// 片方だけならその辺に全部）。各辺の受け持ち区間の積分を、節点の間で線形な分布の積分に等しいとおいて
    /// 節点の値を決める（三重対角の連立）。節点の間隔が違う所でも分布が偏らず、分布の積分は節点の反力の和と
    /// 厳密に一致する
    fn distribute_reactions(&mut self, reaction: &[f64]) {
        // 辺ごとの受け持ち区間の積分（節点の反力そのもの）
        let mut forces: [Vec<f64>; 4] = Default::default();
        for (slot, edge) in forces.iter_mut().zip(Edge::ALL) {
            if self.supports.at(edge) != Support::Free {
                *slot = self
                    .edge_nodes(edge)
                    .iter()
                    .map(|&node| reaction[4 * node])
                    .collect();
            }
        }
        let slot_of = |edge: Edge| Edge::ALL.iter().position(|&e| e == edge).expect("4 辺");
        for corner in Corner::ALL {
            let (ex, ey) = corner.edges();
            let (sx, sy) = (
                self.supports.at(ex) != Support::Free,
                self.supports.at(ey) != Support::Free,
            );
            if !sx && !sy {
                continue;
            }
            let (i, j) = self.corner_node(corner);
            let total = reaction[4 * self.node(i, j)] + self.corner_force(corner);
            // 隅の節点の、各辺の並びの中での位置と、端の要素の半長
            let (kx, ky) = (
                if j == 0 { 0 } else { self.ny() },
                if i == 0 { 0 } else { self.nx() },
            );
            let half_x = self.hy(if j == 0 { 0 } else { self.ny() - 1 });
            let half_y = self.hx(if i == 0 { 0 } else { self.nx() - 1 });
            // 隣の節点の受け持ち長さあたりの反力
            let neighbour = |edge: Edge, end: usize| {
                let positions = self.edge_positions(edge);
                let k = if end == 0 { 1 } else { end - 1 };
                let tributary = (positions[k + 1] - positions[k - 1]) / 2.0;
                (forces[slot_of(edge)][k] / tributary).abs()
            };
            let share_x = if sx { neighbour(ex, kx) * half_x } else { 0.0 };
            let share_y = if sy { neighbour(ey, ky) * half_y } else { 0.0 };
            let (fx, fy) = if share_x + share_y > 0.0 {
                (share_x / (share_x + share_y), share_y / (share_x + share_y))
            } else if sx && sy {
                (0.5, 0.5)
            } else if sx {
                (1.0, 0.0)
            } else {
                (0.0, 1.0)
            };
            if sx {
                forces[slot_of(ex)][kx] = total * fx;
            }
            if sy {
                forces[slot_of(ey)][ky] = total * fy;
            }
        }
        for (slot, edge) in forces.into_iter().zip(Edge::ALL) {
            if !slot.is_empty() {
                let positions = self.edge_positions(edge);
                self.edge_reactions[slot_of(edge)] = piecewise_linear_from_cells(positions, &slot);
            }
        }
    }

    /// 要素 (i, j) の局所座標 (ξ, η) での、x について dx 階、y について dy 階のたわみの導関数
    fn derivative(&self, i: usize, j: usize, xi: f64, eta: f64, dx: usize, dy: usize) -> f64 {
        let f = hermite(self.hx(i), xi);
        let g = hermite(self.hy(j), eta);
        self.element_dofs(i, j)
            .iter()
            .enumerate()
            .map(|(e, &d)| {
                let (m, n, _, _) = split(e);
                self.u[d] * f[m][dx] * g[n][dy]
            })
            .sum()
    }

    /// 座標 x を含む要素の番号と、その要素での局所座標。要素の境界の上（丸めの範囲）なら両隣
    fn containing(positions: &[f64], x: f64) -> Vec<(usize, f64)> {
        let n = positions.len() - 1;
        let k = positions.partition_point(|&p| p <= x).clamp(1, n) - 1;
        let local = |k: usize| 2.0 * (x - positions[k]) / (positions[k + 1] - positions[k]) - 1.0;
        let mut out = vec![(k, local(k))];
        let tolerance = 1e-9 * positions[n].abs();
        if k > 0 && (x - positions[k]).abs() <= tolerance {
            out.push((k - 1, 1.0));
        } else if k + 1 < n && (x - positions[k + 1]).abs() <= tolerance {
            out.push((k + 1, -1.0));
        }
        out
    }

    /// 点 (x, y) を含む要素とそこでの局所座標。要素の境界の上の点は接する要素すべて
    fn elements_at(&self, x: f64, y: f64) -> Vec<(usize, usize, f64, f64)> {
        let cols = Self::containing(&self.xs, x);
        let rows = Self::containing(&self.ys, y);
        cols.iter()
            .flat_map(|&(i, xi)| rows.iter().map(move |&(j, eta)| (i, j, xi, eta)))
            .collect()
    }

    /// 点 (x, y) での導関数を、点を含む要素で評価して平均する（要素の中では解析的、境界の上では跳びの平均）
    fn averaged(&self, x: f64, y: f64, dx: usize, dy: usize) -> f64 {
        let elements = self.elements_at(x, y);
        let sum: f64 = elements
            .iter()
            .map(|&(i, j, xi, eta)| self.derivative(i, j, xi, eta, dx, dy))
            .sum();
        sum / elements.len() as f64
    }

    /// 要素の間で跳ぶ向きに沿って、要素の中央の値を結んで求めた導関数。`along_x` が真なら x 方向に跳ぶ
    /// （w_xxx と w_xxy）。隣り合う 2 つの中央の間は線形に、端の要素の中央より外（辺の上）は端の 3 つの
    /// 中央を通る 2 次式で外挿する
    fn recovered(&self, x: f64, y: f64, dx: usize, dy: usize, along_x: bool) -> f64 {
        let (positions, coord) = if along_x {
            (&self.xs, x)
        } else {
            (&self.ys, y)
        };
        let n = positions.len() - 1;
        let centre = |k: usize| (positions[k] + positions[k + 1]) / 2.0;
        let at_centre = |k: usize| {
            let (cx, cy) = if along_x {
                (centre(k), y)
            } else {
                (x, centre(k))
            };
            let (i, j, xi, eta) = self.elements_at(cx, cy)[0];
            self.derivative(i, j, xi, eta, dx, dy)
        };
        if coord < centre(0) || coord > centre(n - 1) {
            let k0 = if coord < centre(0) { 0 } else { n - 3 };
            let c = [centre(k0), centre(k0 + 1), centre(k0 + 2)];
            let v = [at_centre(k0), at_centre(k0 + 1), at_centre(k0 + 2)];
            return (0..3)
                .map(|m| {
                    let others: Vec<usize> = (0..3).filter(|&o| o != m).collect();
                    v[m] * others
                        .iter()
                        .map(|&o| (coord - c[o]) / (c[m] - c[o]))
                        .product::<f64>()
                })
                .sum();
        }
        let k = (1..n)
            .position(|k| coord <= centre(k))
            .expect("中央の間にある");
        let t = (coord - centre(k)) / (centre(k + 1) - centre(k));
        (1.0 - t) * at_centre(k) + t * at_centre(k + 1)
    }

    /// 要素の間で跳ぶ 2 階の導関数（w_xx は x 方向、w_yy は y 方向に跳ぶ）。要素の中央の値を、両隣の中央との
    /// 2 階の差分で補正し（3 次の補間の 2 階導関数は要素の中央で、要素の幅 h に対して h²/24 × 4 階の導関数
    /// だけ大きい）、点を挟む 4 つの中央を通る 3 次式で結ぶ。端の要素の中央より外（辺の上）は端の 4 つで外挿
    fn recovered_second(&self, x: f64, y: f64, dx: usize, dy: usize, along_x: bool) -> f64 {
        let (positions, coord) = if along_x {
            (&self.xs, x)
        } else {
            (&self.ys, y)
        };
        let n = positions.len() - 1;
        let centre = |k: usize| (positions[k] + positions[k + 1]) / 2.0;
        let sample = |k: usize| {
            let (cx, cy) = if along_x {
                (centre(k), y)
            } else {
                (x, centre(k))
            };
            let (i, j, xi, eta) = self.elements_at(cx, cy)[0];
            self.derivative(i, j, xi, eta, dx, dy)
        };
        let corrected = |k: usize| {
            let (a, b, c) = match k {
                0 => (0, 1, 2),
                k if k == n - 1 => (n - 3, n - 2, n - 1),
                k => (k - 1, k, k + 1),
            };
            let (ma, mb, mc) = (centre(a), centre(b), centre(c));
            let (sa, sb, sc) = (sample(a), sample(b), sample(c));
            let fourth = 2.0 * ((sc - sb) / (mc - mb) - (sb - sa) / (mb - ma)) / (mc - ma);
            let width = positions[k + 1] - positions[k];
            sample(k) - width * width / 24.0 * fourth
        };
        let below = (0..n).rev().find(|&k| centre(k) <= coord).unwrap_or(0);
        let k0 = below.saturating_sub(1).min(n - 4);
        let stencil: Vec<(f64, f64)> = (k0..k0 + 4).map(|k| (centre(k), corrected(k))).collect();
        stencil
            .iter()
            .enumerate()
            .map(|(m, &(cm, vm))| {
                vm * stencil
                    .iter()
                    .enumerate()
                    .filter(|&(o, _)| o != m)
                    .map(|(_, &(co, _))| (coord - co) / (cm - co))
                    .product::<f64>()
            })
            .sum()
    }

    fn derivatives(&self, x: f64, y: f64) -> Derivatives {
        Derivatives {
            w: self.averaged(x, y, 0, 0),
            w_xx: self.recovered_second(x, y, 2, 0, true),
            w_yy: self.recovered_second(x, y, 0, 2, false),
            w_xy: self.averaged(x, y, 1, 1),
            w_xxx: self.recovered(x, y, 3, 0, true),
            w_xxy: self.recovered(x, y, 2, 1, true),
            w_xyy: self.recovered(x, y, 1, 2, false),
            w_yyy: self.recovered(x, y, 0, 3, false),
        }
    }
}

impl Solver for FiniteElement {
    fn interior(&self, x: f64, y: f64) -> Derivatives {
        self.derivatives(x, y)
    }

    fn edge(&self, edge: Edge, along: f64) -> Derivatives {
        let (x, y) = match edge {
            Edge::X0 => (-1.0, along),
            Edge::XLx => (1.0, along),
            Edge::Y0 => (along, -self.b()),
            Edge::YLy => (along, self.b()),
        };
        self.derivatives(x, y)
    }

    fn edge_reaction(&self, edge: Edge, along: f64, _nu: f64) -> f64 {
        let slot = Edge::ALL.iter().position(|&e| e == edge).expect("4 辺");
        let v = &self.edge_reactions[slot];
        if v.is_empty() {
            return 0.0;
        }
        // 節点の間は線形につなぐ
        let positions = self.edge_positions(edge);
        let (k, local) = Self::containing(positions, along)[0];
        let t = (local + 1.0) / 2.0;
        (1.0 - t) * v[k] + t * v[k + 1]
    }

    fn edge_reaction_total(&self, edge: Edge, _half_along: f64, _nu: f64) -> f64 {
        // 節点の間で線形なので、節点での台形則が厳密。値は節点の反力の和に等しい
        let v = &self.edge_reactions[Edge::ALL.iter().position(|&e| e == edge).expect("4 辺")];
        let positions = self.edge_positions(edge);
        v.windows(2)
            .zip(positions.windows(2))
            .map(|(v, s)| (v[0] + v[1]) / 2.0 * (s[1] - s[0]))
            .sum()
    }

    fn corner_w_xy(&self, corner: Corner) -> f64 {
        let (ex, ey) = corner.edges();
        // 自由辺どうしの隅には支持が無く、隅反力は 0
        if self.supports.at(ex) == Support::Free && self.supports.at(ey) == Support::Free {
            return 0.0;
        }
        let (i, j) = self.corner_node(corner);
        self.u[4 * self.node(i, j) + 3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 要素の剛性に、節点の自由度として与えた多項式のたわみを掛け、ひずみエネルギーの 2 倍を返す
    fn energy(hx: f64, hy: f64, nu: f64, w: impl Fn(f64, f64) -> [f64; 4]) -> f64 {
        let (k, _) = Integrals::new().element(hx, hy, nu);
        let mut u = [0.0; ELEMENT_DOFS];
        for (e, value) in u.iter_mut().enumerate() {
            let (node, kind) = (e / 4, e % 4);
            let x = if node % 2 == 0 { -hx } else { hx };
            let y = if node / 2 == 0 { -hy } else { hy };
            *value = w(x, y)[kind];
        }
        (0..ELEMENT_DOFS)
            .map(|r| u[r] * (0..ELEMENT_DOFS).map(|c| k[r][c] * u[c]).sum::<f64>())
            .sum()
    }

    #[test]
    fn rigid_motions_store_no_energy() {
        // 剛体の変位 w = 1、w = x、w = y
        for w in [
            (|_x: f64, _y: f64| [1.0, 0.0, 0.0, 0.0]) as fn(f64, f64) -> [f64; 4],
            |x, _y| [x, 1.0, 0.0, 0.0],
            |_x, y| [y, 0.0, 1.0, 0.0],
        ] {
            assert!(energy(0.3, 0.7, 0.2, w).abs() < 1e-12);
        }
    }

    #[test]
    fn constant_curvatures_store_the_bending_energy() {
        // w = x²/2 は w_xx = 1。エネルギーの 2 倍 = ∫ 1 dA = 面積。w = y²/2 も同じ。
        // w = xy は w_xy = 1 で 2(1 − ν) × 面積。w = (x² + y²)/2 は 2 + 2ν 倍
        let (hx, hy, nu) = (0.3, 0.7, 0.2);
        let area = 4.0 * hx * hy;
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12 * b.abs().max(1.0);
        assert!(close(
            energy(hx, hy, nu, |x, _| [x * x / 2.0, x, 0.0, 0.0]),
            area
        ));
        assert!(close(
            energy(hx, hy, nu, |_, y| [y * y / 2.0, 0.0, y, 0.0]),
            area
        ));
        assert!(close(
            energy(hx, hy, nu, |x, y| [x * y, y, x, 1.0]),
            2.0 * (1.0 - nu) * area
        ));
        assert!(close(
            energy(hx, hy, nu, |x, y| [(x * x + y * y) / 2.0, x, y, 0.0]),
            (2.0 + 2.0 * nu) * area
        ));
    }

    #[test]
    fn the_nodal_load_sums_to_the_area() {
        let (hx, hy) = (0.3, 0.7);
        let (_, load) = Integrals::new().element(hx, hy, 0.0);
        let total: f64 = (0..ELEMENT_DOFS)
            .filter(|e| e % 4 == 0)
            .map(|e| load[e])
            .sum();
        assert!((total - 4.0 * hx * hy).abs() < 1e-12);
    }

    #[test]
    fn a_linear_distribution_is_recovered_from_its_cell_integrals() {
        // v = 1 + 2s を不等間隔の節点で。受け持ち区間の積分を与えると節点の値が戻る
        let positions = [0.0, 0.1, 0.3, 0.6, 1.0];
        let v = |s: f64| 1.0 + 2.0 * s;
        let integral = |a: f64, b: f64| (v(a) + v(b)) / 2.0 * (b - a);
        let mid = |k: usize| (positions[k] + positions[k + 1]) / 2.0;
        let cells = [
            integral(positions[0], mid(0)),
            integral(mid(0), mid(1)),
            integral(mid(1), mid(2)),
            integral(mid(2), mid(3)),
            integral(mid(3), positions[4]),
        ];
        let recovered = piecewise_linear_from_cells(&positions, &cells);
        for (s, r) in positions.iter().zip(&recovered) {
            assert!((r - v(*s)).abs() < 1e-12, "{recovered:?}");
        }
    }

    #[test]
    fn node_positions_are_symmetric_and_denser_at_the_edges() {
        let (xs, ys) = node_positions(1.0, 8);
        assert_eq!(xs, ys);
        assert_eq!(xs.len(), 9);
        assert!((xs[0] + 1.0).abs() < 1e-15 && (xs[8] - 1.0).abs() < 1e-15);
        assert!(xs[4].abs() < 1e-15);
        for k in 0..=8 {
            assert!((xs[k] + xs[8 - k]).abs() < 1e-15);
        }
        assert!(xs[1] - xs[0] < xs[4] - xs[3]);
    }

    #[test]
    fn the_longer_side_keeps_the_edge_spacing_and_fills_the_middle_evenly() {
        // b = 3: y の両端に x と同じ並びの半分ずつ、間は x の中央の要素の大きさ以下で等間隔
        let (xs, ys) = node_positions(3.0, 8);
        let n = ys.len() - 1;
        assert!((ys[0] + 3.0).abs() < 1e-15 && (ys[n] - 3.0).abs() < 1e-15);
        for k in 0..=4 {
            assert!((ys[k] - (xs[k] - 2.0)).abs() < 1e-15);
            assert!((ys[n - k] - (xs[8 - k] + 2.0)).abs() < 1e-15);
        }
        let middle = xs[5] - xs[4];
        for pair in ys[4..=n - 4].windows(2) {
            assert!(pair[1] - pair[0] <= middle + 1e-15);
        }
        assert!(ys[n / 2].abs() < 1e-15);
        // 向きを変えても同じ並びが x に来る
        let (xs2, ys2) = node_positions(1.0 / 3.0, 8);
        assert_eq!(ys2.len(), 9);
        assert_eq!(xs2.len(), ys.len());
        for (a, b) in xs2.iter().zip(&ys) {
            assert!((a - b / 3.0).abs() < 1e-15);
        }
    }
}
