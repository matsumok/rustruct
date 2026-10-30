//! Kirchhoff 板のテストで共有する部品。板の組み立て、理論から言える値（Navier の二重級数）、2 つの解の差の
//! 取り方。テストの crate ごとに使う部品が違うので、使わないものの警告は出さない
#![allow(dead_code)]

use std::f64::consts::PI;

use rustruct_calc::kirchhoff_plate::{EdgeSample, Plate, PointResponse, Solution, SupportType};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Supports};
use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};

/// 打ち切り・離散化の誤差を許す比較。`tolerance` は絶対値で、呼ぶ側が値と理由を書く
pub fn assert_within(actual: f64, expected: f64, tolerance: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: {actual} と {expected} の差 {} が許容 {tolerance} を超える",
        (actual - expected).abs()
    );
}

/// 辺長は mm、p は kN/m²。t = 100 mm、E = 20 000 N/mm²
pub fn plate(lx: f64, ly: f64, nu: f64, p: f64, supports: Supports) -> Plate {
    Plate {
        lx: Length::mm(lx),
        ly: Length::mm(ly),
        thickness: Length::mm(100.0),
        young_modulus: Stress::n_per_mm2(20_000.0),
        poisson_ratio: PoissonRatio::new(nu).unwrap(),
        load: AreaLoad::kn_per_m2(p),
        supports,
    }
}

/// 曲げ剛性 D = E t³ / (12 (1 − ν²)) [N·mm]
pub fn rigidity(plate: &Plate) -> f64 {
    let t = plate.thickness.as_mm();
    plate.young_modulus.as_n_per_mm2() * t * t * t
        / (12.0 * (1.0 - plate.poisson_ratio.value().powi(2)))
}

pub fn at(s: &Solution, x: f64, y: f64) -> PointResponse {
    s.at(Length::mm(x), Length::mm(y)).expect("板の上の点")
}

/// 辺の反力の台形則の積分 [N]
pub fn integrated_reaction(s: &Solution, edge: Edge, divisions: usize) -> f64 {
    let samples = s.edge(edge, divisions);
    samples
        .windows(2)
        .map(|pair| {
            let ds = pair[1].along.as_mm() - pair[0].along.as_mm();
            (pair[0].reaction.as_n_per_mm() + pair[1].reaction.as_n_per_mm()) / 2.0 * ds
        })
        .sum()
}

/// 辺の反力の合計 − 隅反力の合計 [N]
pub fn reactions_minus_corners(s: &Solution) -> f64 {
    let reactions: f64 = Edge::ALL.iter().map(|&e| s.total_reaction(e).as_n()).sum();
    let corners: f64 = Corner::ALL
        .iter()
        .map(|&c| s.corner_reaction(c).as_n())
        .sum();
    reactions - corners
}

/// 四辺単純支持の板の、隅原点の点 (x, y) の応答と隅の w_xy。Navier の二重正弦級数で、どの解き方とも独立
pub struct Navier {
    pub w: f64,
    pub mx: f64,
    pub my: f64,
    pub qx: f64,
    pub qy: f64,
    pub w_xy: f64,
}

/// m・n は奇数を `max` まで。w = Σ A sin αx sin βy、α = mπ/Lx、β = nπ/Ly、A = 16 p / (π² m n D (α² + β²)²)。
/// 曲げモーメントとせん断力は解き方と同じ定義（M = −D (w_xx + ν w_yy)、Q = −D (w_xxx + w_xyy)）から項ごとに微分する
pub fn navier(plate: &Plate, x: f64, y: f64, max: usize) -> Navier {
    let (lx, ly) = (plate.lx.as_mm(), plate.ly.as_mm());
    let (p, d, nu) = (
        plate.load.as_n_per_mm2(),
        rigidity(plate),
        plate.poisson_ratio.value(),
    );
    let mut r = Navier {
        w: 0.0,
        mx: 0.0,
        my: 0.0,
        qx: 0.0,
        qy: 0.0,
        w_xy: 0.0,
    };
    for m in (1..=max).step_by(2) {
        for n in (1..=max).step_by(2) {
            let alpha = m as f64 * PI / lx;
            let beta = n as f64 * PI / ly;
            let k = alpha * alpha + beta * beta;
            let amp = 16.0 * p / (PI * PI * (m * n) as f64 * d * k * k);
            let (sx, cx) = (alpha * x).sin_cos();
            let (sy, cy) = (beta * y).sin_cos();
            r.w += amp * sx * sy;
            r.mx += d * amp * (alpha * alpha + nu * beta * beta) * sx * sy;
            r.my += d * amp * (beta * beta + nu * alpha * alpha) * sx * sy;
            r.qx += d * amp * alpha * k * cx * sy;
            r.qy += d * amp * beta * k * sx * cy;
            r.w_xy += amp * alpha * beta * cx * cy;
        }
    }
    r
}

/// 隅から辺長の 5 % 以内の点か（x・y ともに辺長の 5 % より端に寄っている）。ADR-0008 で除く範囲
pub fn near_corner(x: f64, y: f64, lx: f64, ly: f64) -> bool {
    !(0.05 * lx..=0.95 * lx).contains(&x) && !(0.05 * ly..=0.95 * ly).contains(&y)
}

/// 解ける 19 型。21 型から不安定な 2 型（四辺自由、単純支持 1 辺と自由 3 辺）を除いたもの
pub fn stable_types() -> impl Iterator<Item = SupportType> {
    SupportType::ALL
        .into_iter()
        .filter(|t| !matches!(t, SupportType::RRRR | SupportType::RRRS))
}

/// 比較の分母。たわみ・曲げモーメント・せん断力それぞれの、21 × 21 のグリッドでの絶対値の最大。隅から辺長の
/// 5 % 以内の点は除く（固定辺と自由辺の隅ではせん断力が発散し、分母にすると比が小さく出る。ADR-0008）
pub struct Scale {
    pub w: f64,
    pub m: f64,
    pub q: f64,
}

impl Scale {
    pub fn of(s: &Solution) -> Self {
        let (lx, ly) = (s.plate().lx.as_mm(), s.plate().ly.as_mm());
        let g: Vec<PointResponse> = s
            .grid(20, 20)
            .into_iter()
            .filter(|p| !near_corner(p.x.as_mm(), p.y.as_mm(), lx, ly))
            .collect();
        let max = |f: Pick| g.iter().map(|p| f(p).abs()).fold(0.0, f64::max);
        Scale {
            w: max(|p| p.w.as_mm()),
            m: max(|p| p.mx.as_nmm_per_mm()).max(max(|p| p.my.as_nmm_per_mm())),
            q: max(|p| p.qx.as_n_per_mm()).max(max(|p| p.qy.as_n_per_mm())),
        }
    }
}

/// 点の応答から 1 つの量を取り出す
pub type Pick = fn(&PointResponse) -> f64;

pub const POINT_QUANTITIES: [(&str, Pick); 5] = [
    ("w", |p| p.w.as_mm()),
    ("Mx", |p| p.mx.as_nmm_per_mm()),
    ("My", |p| p.my.as_nmm_per_mm()),
    ("Qx", |p| p.qx.as_n_per_mm()),
    ("Qy", |p| p.qy.as_n_per_mm()),
];

pub type PickEdge = fn(&EdgeSample) -> f64;

pub const EDGE_QUANTITIES: [(&str, PickEdge); 3] = [
    ("M", |s| s.moment.as_nmm_per_mm()),
    ("Q", |s| s.shear.as_n_per_mm()),
    ("V", |s| s.reaction.as_n_per_mm()),
];

/// 有限要素の照合の許容。たわみと曲げモーメントは 0.5 %、せん断力・辺の反力・隅反力は 2 %
/// （その量の板全体の絶対値の最大に対して。ADR-0002）
pub fn tolerance_of(name: &str) -> f64 {
    if name.starts_with('w') || name.starts_with('M') {
        5e-3
    } else {
        2e-2
    }
}

/// 2 つの解の差。量ごとに、基準の解のその量の絶対値の最大に対する比の最大と、その点
pub struct Deviation {
    pub name: String,
    pub ratio: f64,
    pub at: String,
}

/// 21 × 21 のグリッド（隅から辺長の 5 % 以内を除く）と、各辺の 101 点（両端から 5 % を除く）、隅反力で比べる。
/// 比の分母は量の種類ごと（たわみ、曲げモーメント、せん断力、辺の反力、隅反力）に、基準の解の板全体での
/// 絶対値の最大。単純支持・自由の辺の曲げモーメントや自由辺の反力は厳密に 0 で、板全体の最大と比べる
pub fn deviations(reference: &Solution, other: &Solution) -> Vec<Deviation> {
    let (lx, ly) = (reference.plate().lx.as_mm(), reference.plate().ly.as_mm());
    let mut out = Vec::new();
    let (ga, gb) = (reference.grid(20, 20), other.grid(20, 20));
    let edges: Vec<(Edge, Vec<EdgeSample>, Vec<EdgeSample>)> = Edge::ALL
        .iter()
        .map(|&e| (e, reference.edge(e, 100), other.edge(e, 100)))
        .collect();
    let max = |values: &mut dyn Iterator<Item = f64>| values.map(f64::abs).fold(0.0, f64::max);
    let w_scale = max(&mut ga.iter().map(|p| p.w.as_mm()));
    let m_scale = max(&mut ga
        .iter()
        .flat_map(|p| [p.mx.as_nmm_per_mm(), p.my.as_nmm_per_mm()]));
    let q_scale = max(&mut ga
        .iter()
        .flat_map(|p| [p.qx.as_n_per_mm(), p.qy.as_n_per_mm()])
        .chain(
            edges
                .iter()
                .flat_map(|(_, ea, _)| ea.iter().map(|s| s.shear.as_n_per_mm())),
        ));
    let v_scale = max(&mut edges
        .iter()
        .flat_map(|(_, ea, _)| ea.iter().map(|s| s.reaction.as_n_per_mm())));
    for (name, f) in POINT_QUANTITIES {
        let scale = match name {
            "w" => w_scale,
            "Mx" | "My" => m_scale,
            _ => q_scale,
        };
        let mut worst = Deviation {
            name: name.to_string(),
            ratio: 0.0,
            at: String::new(),
        };
        for (pa, pb) in ga.iter().zip(&gb) {
            let (x, y) = (pa.x.as_mm(), pa.y.as_mm());
            if near_corner(x, y, lx, ly) {
                continue;
            }
            let r = (f(pa) - f(pb)).abs() / scale;
            if r > worst.ratio {
                worst.ratio = r;
                worst.at = format!("({x}, {y})");
            }
        }
        out.push(worst);
    }
    for (e, ea, eb) in &edges {
        for (name, f) in EDGE_QUANTITIES {
            let scale = match name {
                "M" => m_scale,
                "Q" => q_scale,
                _ => v_scale,
            };
            let mut worst = Deviation {
                name: format!("{e} の {name}"),
                ratio: 0.0,
                at: String::new(),
            };
            for (sa, sb) in ea.iter().zip(eb).skip(5).take(91) {
                let r = (f(sa) - f(sb)).abs() / scale;
                if r > worst.ratio {
                    worst.ratio = r;
                    worst.at = format!("{}", sa.along.as_mm());
                }
            }
            out.push(worst);
        }
    }
    let scale = max(&mut Corner::ALL
        .iter()
        .map(|&c| reference.corner_reaction(c).as_n()));
    if scale > 0.0 {
        for c in Corner::ALL {
            let r = (reference.corner_reaction(c).as_n() - other.corner_reaction(c).as_n()).abs()
                / scale;
            out.push(Deviation {
                name: "隅反力".to_string(),
                ratio: r,
                at: format!("{c:?}"),
            });
        }
    }
    out
}

pub fn assert_deviations_within_tolerance(reference: &Solution, other: &Solution, what: &str) {
    for d in deviations(reference, other) {
        let tolerance = tolerance_of(&d.name);
        assert!(
            d.ratio <= tolerance,
            "{what} の {}: 差 {:.2e} が許容 {tolerance} を超える（{}）",
            d.name,
            d.ratio,
            d.at
        );
    }
}
