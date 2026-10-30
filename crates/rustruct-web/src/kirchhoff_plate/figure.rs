//! 分布図: 板全面の 1 つの量を、等値線と帯で示す図（`docs/adr/0011`）。
//!
//! 計算核の等間隔グリッドの応答（[`PlateField`]）から 1 つの量のスカラー場を取り出し、`charta` の
//! 分布図にする。図は板の縦横比どおりに 1 単位（`width-column`）の正方形に収め、周りの縁に 4 辺の支持条件を
//! 描く: 固定は辺の線と外側の斜線、単純支持は辺の線だけ、自由は線を引かない。色はライトとダークの 2 枚を
//! 作り、ページがテーマで片方を見せる。

use charta::{Grid, Item, Palette, Pt, Scene, Stroke, Theme, distribution, to_svg};
use rustruct_calc::kirchhoff_plate::{PointResponse, Solution, solve};
use rustruct_types::kirchhoff_plate::{Edge, Support, Supports};

use super::{EDGES, PlateInputs, support_name};
use crate::COLUMN_WIDTH;

/// 長い辺を等分する評価点の間の数。短い辺は辺長の比で決め、評価点の間はほぼ正方形になる
const GRID_DIVISIONS: usize = 60;
/// 短い辺を等分する数の下限（帯板に近い板でも、短い辺を横切る山が描ける数）
const MIN_DIVISIONS: usize = 8;
/// 板の周りの縁の幅。固定辺の斜線を描く
const MARGIN: f64 = 6.0;
/// 固定辺の斜線の長さ（辺に直交する向きの成分）と間隔
const HATCH: f64 = 5.0;
/// 辺の線の太さ
const EDGE_WIDTH: f64 = 1.0;
/// 固定辺の斜線の太さ
const HATCH_WIDTH: f64 = 0.5;

/// 分布図に描く量。並びは切替の順
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    W,
    Mx,
    My,
    Qx,
    Qy,
}

impl Quantity {
    pub const ALL: [Self; 5] = [Self::W, Self::Mx, Self::My, Self::Qx, Self::Qy];

    /// 切替の文字
    pub fn symbol(self) -> &'static str {
        match self {
            Self::W => "w",
            Self::Mx => "Mx",
            Self::My => "My",
            Self::Qx => "Qx",
            Self::Qy => "Qy",
        }
    }

    /// 量の名前
    fn name(self) -> &'static str {
        match self {
            Self::W => "たわみ",
            Self::Mx | Self::My => "曲げモーメント",
            Self::Qx | Self::Qy => "せん断力",
        }
    }

    /// 図の名前。単位は括弧で
    pub fn caption(self) -> &'static str {
        match self {
            Self::W => "たわみ w（mm）",
            Self::Mx => "曲げモーメント Mx（kN·m/m）",
            Self::My => "曲げモーメント My（kN·m/m）",
            Self::Qx => "せん断力 Qx（kN/m）",
            Self::Qy => "せん断力 Qy（kN/m）",
        }
    }

    /// 帯の色。正負のない量（たわみ）は `seq`、正負のある量は `div`（`docs/adr/0011`）
    fn palette(self) -> Palette {
        match self {
            Self::W => Palette::Seq,
            _ => Palette::Div,
        }
    }

    /// 図の名前の単位での値
    fn value(self, r: &PointResponse) -> f64 {
        match self {
            Self::W => r.w.as_mm(),
            Self::Mx => r.mx.as_knm_per_m(),
            Self::My => r.my.as_knm_per_m(),
            Self::Qx => r.qx.as_kn_per_m(),
            Self::Qy => r.qy.as_kn_per_m(),
        }
    }
}

/// 解から取った、等間隔の評価点の応答。量を切り替えても解き直さないように、入力から一度だけ作る
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlateField {
    /// 辺長（m）
    lx: f64,
    ly: f64,
    supports: Supports,
    nx: usize,
    ny: usize,
    /// y の小さい行から、行の中は x の小さい順（`Solution::grid`）
    points: Vec<PointResponse>,
}

impl PlateField {
    pub(crate) fn new(solution: &Solution) -> Self {
        let plate = solution.plate();
        let (lx, ly) = (plate.lx.as_mm() / 1000.0, plate.ly.as_mm() / 1000.0);
        let short = |a: f64, b: f64| {
            ((GRID_DIVISIONS as f64 * a / b).round() as usize).clamp(MIN_DIVISIONS, GRID_DIVISIONS)
        };
        let (nx, ny) = if lx >= ly {
            (GRID_DIVISIONS, short(ly, lx))
        } else {
            (short(lx, ly), GRID_DIVISIONS)
        };
        Self {
            lx,
            ly,
            supports: plate.supports,
            nx,
            ny,
            points: solution.grid(nx, ny),
        }
    }
}

/// 1 つの量の分布図
#[derive(Debug, Clone, PartialEq)]
pub struct PlateFigure {
    /// 図の名前（単位を括弧で）
    pub caption: &'static str,
    /// 読み上げの文言。量と 4 辺の支持条件
    pub label: String,
    pub light_svg: String,
    pub dark_svg: String,
}

impl PlateFigure {
    /// 入力から。入力が読めないか、板を支えられない組なら図は無い
    pub fn from_inputs(inputs: &PlateInputs, quantity: Quantity) -> Option<Self> {
        let solution = solve(&inputs.plate().ok()?).ok()?;
        Some(Self::new(&PlateField::new(&solution), quantity))
    }

    pub(crate) fn new(field: &PlateField, quantity: Quantity) -> Self {
        let scene = scene(field, quantity);
        let svg = |theme| to_svg(&scene, &theme).expect("役割名は tokens.json にある");
        let supports = EDGES
            .map(|(edge, _)| format!("{edge} の辺は{}", support_name(field.supports.at(edge))))
            .join("、");
        Self {
            caption: quantity.caption(),
            label: format!(
                "{} {} の分布図。{supports}",
                quantity.name(),
                quantity.symbol()
            ),
            light_svg: svg(Theme::light()),
            dark_svg: svg(Theme::dark()),
        }
    }
}

/// 板の分布図を縁の内側に置き、縁に支持条件を描く
fn scene(field: &PlateField, quantity: Quantity) -> Scene {
    let z = field.points.iter().map(|r| quantity.value(r)).collect();
    let grid = Grid::new(
        Grid::axis(field.lx, field.nx),
        Grid::axis(field.ly, field.ny),
        z,
    )
    .expect("評価点の数は分け方どおり");
    let scale = (COLUMN_WIDTH - 2.0 * MARGIN) / field.lx.max(field.ly);
    let (width, height) = (field.lx * scale, field.ly * scale);
    let palette = quantity.palette();

    let mut scene = Scene::new(width + 2.0 * MARGIN, height + 2.0 * MARGIN);
    scene.place(
        distribution(&grid, palette, palette.default_steps(), width),
        [MARGIN, MARGIN],
    );
    let (left, right, top, bottom) = (MARGIN, MARGIN + width, MARGIN, MARGIN + height);
    for (edge, _) in EDGES {
        // 辺の両端（y は上が正なので、y = 0 の辺が下）と、板の外への向き
        let (from, to, outward): (Pt, Pt, Pt) = match edge {
            Edge::X0 => ([left, bottom], [left, top], [-1.0, 0.0]),
            Edge::XLx => ([right, bottom], [right, top], [1.0, 0.0]),
            Edge::Y0 => ([left, bottom], [right, bottom], [0.0, 1.0]),
            Edge::YLy => ([left, top], [right, top], [0.0, -1.0]),
        };
        let support = field.supports.at(edge);
        if support == Support::Free {
            continue;
        }
        scene.items.push(Item::Path {
            subpaths: vec![vec![from, to]],
            closed: false,
            fill: None,
            stroke: Some(Stroke::new("ink", EDGE_WIDTH)),
        });
        if support == Support::Fixed {
            scene.items.push(Item::Path {
                subpaths: hatch(from, to, outward),
                closed: false,
                fill: None,
                stroke: Some(Stroke::new("ink", HATCH_WIDTH)),
            });
        }
    }
    scene
}

/// 辺 `from`→`to` の外側の斜線。辺に沿って `HATCH` おきに、外へ斜め 45° に引く
fn hatch(from: Pt, to: Pt, outward: Pt) -> Vec<Vec<Pt>> {
    let length = (to[0] - from[0]).hypot(to[1] - from[1]);
    let along = [(to[0] - from[0]) / length, (to[1] - from[1]) / length];
    let count = (length / HATCH).floor() as usize;
    let start = (length - count as f64 * HATCH) / 2.0;
    (0..=count)
        .map(|k| {
            let s = start + k as f64 * HATCH;
            let p = [from[0] + along[0] * s, from[1] + along[1] * s];
            vec![
                p,
                [
                    p[0] + (outward[0] - along[0]) * HATCH,
                    p[1] + (outward[1] - along[1]) * HATCH,
                ],
            ]
        })
        .collect()
}
