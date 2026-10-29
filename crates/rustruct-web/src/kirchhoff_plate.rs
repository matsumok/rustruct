//! Tool「Kirchhoff 板の曲げ」（`/tools/kirchhoff-plate`）。言葉は `docs/tools/kirchhoff-plate/GLOSSARY.md`、
//! 符号と座標は同じ所の ADR-0004。数値はすべて `rustruct-calc` の `solve` から引き、ここでは入力の文字の
//! 読み取りと、表示の桁だけを決める。計算はブラウザの中で同期に回し、入力を変えるたびに結果を描き直す。
//!
//! 辺は座標で呼ぶ（x = 0 の辺など）。画面の上下左右は図の向き（x は右、y は上）でここが決め、入力の
//! Sheet の行見出しに添えて対応を示す。

use dioxus::prelude::*;
use rustruct_calc::kirchhoff_plate::{PlateError, Solution, solve};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Plate, Support, Supports};
use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};

use crate::table::{
    Block, Caption, Column, Columns, DecimalCell, HeadCell, HeadRow, Notes, Row, Table, TextCell,
    max_decimals, significant, thousands,
};
use crate::{Page, Route};

/// 計算できない値の表示（`docs/ui-decisions.md`「エラーと警告」）
const NOT_COMPUTED: &str = "—";

/// 辺の最大値を取る評価点の分け方。辺を等分する数
const EDGE_DIVISIONS: usize = 100;

/// 入力の Sheet の欄。並びは Sheet の行の順
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    Lx,
    Ly,
    Thickness,
    YoungModulus,
    PoissonRatio,
    Load,
}

impl InputField {
    const ALL: [Self; 6] = [
        Self::Lx,
        Self::Ly,
        Self::Thickness,
        Self::YoungModulus,
        Self::PoissonRatio,
        Self::Load,
    ];

    /// 欄の名前。単位は括弧で名前に付ける
    pub fn label(self) -> &'static str {
        match self {
            Self::Lx => "Lx（m）",
            Self::Ly => "Ly（m）",
            Self::Thickness => "t（mm）",
            Self::YoungModulus => "E（N/mm²）",
            Self::PoissonRatio => "ν",
            Self::Load => "p（kN/m²）",
        }
    }
}

/// 辺の画面での呼び方。図の向き（x は右、y は上）で決める。並びは入力の Sheet と結果の表の行の順
const EDGES: [(Edge, &str); 4] = [
    (Edge::X0, "左"),
    (Edge::XLx, "右"),
    (Edge::Y0, "下"),
    (Edge::YLy, "上"),
];

/// 入力の Sheet の 1 行に並べる、対になる欄
const FIELD_PAIRS: [[InputField; 2]; 3] = [
    [InputField::Lx, InputField::Ly],
    [InputField::Thickness, InputField::YoungModulus],
    [InputField::PoissonRatio, InputField::Load],
];

/// 入力の Sheet の 1 行に並べる、向かい合う辺
const EDGE_PAIRS: [[(Edge, &str); 2]; 2] = [[EDGES[0], EDGES[1]], [EDGES[2], EDGES[3]]];

/// 支持条件の選び方。並びは選択肢の順
const SUPPORTS: [(Support, &str); 3] = [
    (Support::Fixed, "固定"),
    (Support::SimplySupported, "単純支持"),
    (Support::Free, "自由"),
];

/// 入力の Sheet の中身。数値の欄は打ったままの文字で持ち、読めるかは `PlateReadout` が決める
#[derive(Debug, Clone, PartialEq)]
pub struct PlateInputs {
    pub lx: String,
    pub ly: String,
    pub thickness: String,
    pub young_modulus: String,
    pub poisson_ratio: String,
    pub load: String,
    pub supports: Supports,
}

impl Default for PlateInputs {
    /// 開いたときの値（Issue #36）
    fn default() -> Self {
        Self {
            lx: "4.0".into(),
            ly: "5.0".into(),
            thickness: "150".into(),
            young_modulus: "22700".into(),
            poisson_ratio: "0".into(),
            load: "10".into(),
            supports: Supports::all(Support::Fixed),
        }
    }
}

impl PlateInputs {
    fn get(&self, field: InputField) -> &str {
        match field {
            InputField::Lx => &self.lx,
            InputField::Ly => &self.ly,
            InputField::Thickness => &self.thickness,
            InputField::YoungModulus => &self.young_modulus,
            InputField::PoissonRatio => &self.poisson_ratio,
            InputField::Load => &self.load,
        }
    }

    fn set(&mut self, field: InputField, value: String) {
        *match field {
            InputField::Lx => &mut self.lx,
            InputField::Ly => &mut self.ly,
            InputField::Thickness => &mut self.thickness,
            InputField::YoungModulus => &mut self.young_modulus,
            InputField::PoissonRatio => &mut self.poisson_ratio,
            InputField::Load => &mut self.load,
        } = value;
    }

    /// Model へ。読めない欄はすべて（欄, 理由）で返す
    fn plate(&self) -> Result<Plate, Vec<(InputField, &'static str)>> {
        let mut errors = Vec::new();
        let mut read = |field: InputField| match read_value(field, self.get(field)) {
            Ok(v) => v,
            Err(message) => {
                errors.push((field, message));
                f64::NAN
            }
        };
        let [lx, ly, thickness, young_modulus, poisson_ratio, load] =
            InputField::ALL.map(&mut read);
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Plate {
            lx: Length::mm(lx * 1000.0),
            ly: Length::mm(ly * 1000.0),
            thickness: Length::mm(thickness),
            young_modulus: Stress::n_per_mm2(young_modulus),
            poisson_ratio: PoissonRatio::new(poisson_ratio).expect("範囲は read_value で見た"),
            load: AreaLoad::kn_per_m2(load),
            supports: self.supports,
        })
    }
}

/// 1 つの欄を数に読む。3 桁区切りの「,」は読み飛ばす。読めない理由は欄に添える文言
fn read_value(field: InputField, input: &str) -> Result<f64, &'static str> {
    let input = input.trim().replace(',', "");
    if input.is_empty() {
        return Err("値を入れる");
    }
    let value: f64 = input
        .parse()
        .ok()
        .filter(|v: &f64| v.is_finite())
        .ok_or("数値で入れる")?;
    match field {
        InputField::Lx | InputField::Ly | InputField::Thickness | InputField::YoungModulus
            if value <= 0.0 =>
        {
            Err("0 より大きい値を入れる")
        }
        InputField::PoissonRatio if PoissonRatio::new(value).is_err() => {
            Err("0 以上 0.5 未満で入れる")
        }
        _ => Ok(value),
    }
}

/// 入力から求めた、結果の表の表示。数値は丸めて文字にしたもの。計算できないときは「—」
#[derive(Debug, Clone, PartialEq)]
pub struct PlateReadout {
    /// 読めない欄と、その理由
    pub errors: Vec<(InputField, &'static str)>,
    /// 支持条件の組を解けない理由（不安定か、未対応か）。「!」を付けて出す
    pub notice: Option<&'static str>,
    /// 板の中央の値
    pub center: CenterRow,
    /// 辺ごとの行。`EDGES` の順
    pub edges: Vec<EdgeRow>,
    /// 隅ごとの隅反力（kN）。`Corner::ALL` の順
    pub corners: Vec<(String, String)>,
    /// 釣り合い
    pub balance: Balance,
}

/// 結果の表の、辺の 1 行
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeRow {
    /// 辺の名前（x = 0 など）
    pub name: String,
    /// 辺に直交する方向の曲げモーメントの辺の最大値
    pub max_moment: Maximum,
    /// せん断力の辺の最大値
    pub max_shear: Maximum,
    /// 辺の反力の辺の最大値
    pub max_reaction: Maximum,
}

/// 結果の表の、板の中央の行
#[derive(Debug, Clone, PartialEq)]
pub struct CenterRow {
    /// たわみ（mm）
    pub w: String,
    /// 曲げモーメント（kN·m/m）
    pub mx: String,
    pub my: String,
}

/// 辺の最大値の 1 つ
#[derive(Debug, Clone, PartialEq)]
pub struct Maximum {
    /// 絶対値が最大の値（曲げモーメントは kN·m/m、せん断力と辺の反力は kN/m）
    pub value: String,
    /// 辺の始点からの位置（m）。値が辺の全体で 0 のときは「—」
    pub along: String,
}

impl Maximum {
    fn new(value: f64, along: Length) -> Self {
        Self {
            value: figures(value),
            along: if value == 0.0 {
                NOT_COMPUTED.into()
            } else {
                figures(along.as_mm() / 1000.0)
            },
        }
    }

    fn not_computed() -> Self {
        Self {
            value: NOT_COMPUTED.into(),
            along: NOT_COMPUTED.into(),
        }
    }
}

/// 釣り合いの行（kN）
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    /// 辺の反力の合計 − 隅反力の合計
    pub reactions: String,
    /// 荷重の合計
    pub load: String,
}

impl PlateReadout {
    pub fn from_inputs(inputs: &PlateInputs) -> Self {
        match inputs.plate() {
            Err(errors) => Self {
                errors,
                ..Self::not_computed(None)
            },
            Ok(plate) => match solve(&plate) {
                Ok(solution) => Self::computed(&solution),
                Err(PlateError::Unstable(_)) => {
                    Self::not_computed(Some("この支持条件の組では板を支えられない"))
                }
                // 入力は手前で読んだので来ない。来たら解けないことだけを言う
                Err(PlateError::InvalidInput(_) | PlateError::Singular) => {
                    Self::not_computed(Some("この入力では解けない"))
                }
            },
        }
    }

    fn not_computed(notice: Option<&'static str>) -> Self {
        let dash = || NOT_COMPUTED.to_string();
        Self {
            errors: Vec::new(),
            notice,
            center: CenterRow {
                w: dash(),
                mx: dash(),
                my: dash(),
            },
            edges: EDGES
                .map(|(edge, _)| EdgeRow {
                    name: edge.to_string(),
                    max_moment: Maximum::not_computed(),
                    max_shear: Maximum::not_computed(),
                    max_reaction: Maximum::not_computed(),
                })
                .to_vec(),
            corners: Corner::ALL
                .map(|corner| (corner_name(corner), dash()))
                .to_vec(),
            balance: Balance {
                reactions: dash(),
                load: dash(),
            },
        }
    }

    fn computed(solution: &Solution) -> Self {
        let plate = solution.plate();
        let (lx, ly) = (plate.lx.as_mm(), plate.ly.as_mm());
        let total_load = plate.load.as_kn_per_m2() * lx * ly / 1e6;

        let center = solution
            .at(Length::mm(lx / 2.0), Length::mm(ly / 2.0))
            .expect("中央は板の上");
        let mut edge_total = 0.0;
        let edges = EDGES
            .map(|(edge, _)| {
                edge_total += solution.total_reaction(edge).as_kn();
                let max = solution.edge_maximum(edge, EDGE_DIVISIONS);
                EdgeRow {
                    name: edge.to_string(),
                    max_moment: Maximum::new(max.moment.value.as_knm_per_m(), max.moment.along),
                    max_shear: Maximum::new(max.shear.value.as_kn_per_m(), max.shear.along),
                    max_reaction: Maximum::new(
                        max.reaction.value.as_kn_per_m(),
                        max.reaction.along,
                    ),
                }
            })
            .to_vec();
        let corner_reactions = Corner::ALL.map(|corner| solution.corner_reaction(corner).as_kn());
        Self {
            errors: Vec::new(),
            notice: None,
            center: CenterRow {
                w: figures(center.w.as_mm()),
                mx: figures(center.mx.as_knm_per_m()),
                my: figures(center.my.as_knm_per_m()),
            },
            edges,
            corners: Corner::ALL
                .into_iter()
                .zip(corner_reactions)
                .map(|(corner, r)| (corner_name(corner), figures(r)))
                .collect(),
            balance: Balance {
                reactions: figures(edge_total - corner_reactions.iter().sum::<f64>()),
                load: figures(total_load),
            },
        }
    }
}

/// 1 つの辺の支持条件を選び直す
fn set_support(supports: &mut Supports, edge: Edge, support: Support) {
    *match edge {
        Edge::X0 => &mut supports.x0,
        Edge::Y0 => &mut supports.y0,
        Edge::XLx => &mut supports.x_lx,
        Edge::YLy => &mut supports.y_ly,
    } = support;
}

/// 隅の名前。会う 2 辺で呼ぶ（x = 0・y = 0）
fn corner_name(corner: Corner) -> String {
    let (x, y) = corner.edges();
    format!("{x}・{y}")
}

/// 有効数字 3 桁、4 桁以上は 3 桁区切り（Data と同じ）
fn figures(value: f64) -> String {
    thousands(significant(value, 3))
}

/// `/tools/kirchhoff-plate`。入力の Sheet のすぐ下に結果の表を続け、1 つの列に置く（ADR-0006）。
/// 「入力」「結果」の部の見出しは置かず、表の見出しで区切る。
#[component]
pub(crate) fn KirchhoffPlatePage() -> Element {
    let inputs = use_signal(PlateInputs::default);
    let readout = PlateReadout::from_inputs(&inputs.read());
    let CenterRow { w, mx, my } = readout.center.clone();
    let corner_decimals = max_decimals(readout.corners.iter().map(|(_, r)| r.as_str()));
    let Balance { reactions, load } = readout.balance.clone();
    rsx! {
        Page { route: Route::ToolsKirchhoffPlate {},
            Columns {
                Column {
                    // 入力のすぐ下に結果（`space-6`）。この組と次の表の間は Column の間（`space-7`）
                    div { class: "flex flex-col gap-6",
                        div { class: "flex flex-col gap-1",
                            InputSheet { inputs, errors: readout.errors.clone() }
                            Notes {
                                li { "ν は 0 を基本とする" }
                            }
                            for (field , message) in readout.errors.iter().copied() {
                                p { class: "text-label text-ng", "{field.label()}: {message}" }
                            }
                            if let Some(notice) = readout.notice {
                                p { class: "text-label", "! {notice}" }
                            }
                        }
                        Block {
                            Caption { "板の中央" }
                            Table { fixed: true,
                                thead {
                                    HeadRow {
                                        HeadCell { "w（mm）" }
                                        HeadCell { "Mx（kN·m/m）" }
                                        HeadCell { "My（kN·m/m）" }
                                    }
                                }
                                tbody {
                                    Row {
                                        DecimalCell { value: w, decimals: 0 }
                                        DecimalCell { value: mx, decimals: 0 }
                                        DecimalCell { value: my, decimals: 0 }
                                    }
                                }
                            }
                            Notes {
                                li { "たわみは下向きを正とする" }
                                li { "曲げモーメントは下側が引張になる向きを正とする" }
                            }
                        }
                    }
                    Block {
                        Caption { "辺の最大値" }
                        EdgeTable { edges: readout.edges.clone(), positions: false }
                        Notes {
                            li { "辺上の絶対値の最大値" }
                            li { "M は曲げモーメント、Q はせん断力、V は辺の反力" }
                            li { "左右の辺の M・Q は Mx・Qx、上下の辺は My・Qy" }
                            li { "単純支持辺と自由辺では M は 0" }
                            li { "辺の反力は支える向きを正とする" }
                        }
                    }
                    Block {
                        Caption { "辺の最大値の位置（m）" }
                        EdgeTable { edges: readout.edges.clone(), positions: true }
                        Notes {
                            li { "位置は座標 0 の端からの距離" }
                        }
                    }
                    Block {
                        Caption { "隅反力" }
                        Table { fixed: true,
                            thead {
                                HeadRow {
                                    HeadCell { left: true, "隅" }
                                    HeadCell { "R（kN）" }
                                }
                            }
                            tbody {
                                for (name , reaction) in readout.corners.iter().cloned() {
                                    Row {
                                        TextCell { {name} }
                                        DecimalCell { value: reaction, decimals: corner_decimals }
                                    }
                                }
                            }
                        }
                        Notes {
                            li { "隅反力は板を押さえる向きを正とする" }
                        }
                    }
                    Block {
                        Caption { "釣り合い" }
                        Table { fixed: true,
                            thead {
                                HeadRow {
                                    HeadCell { "ΣV − ΣR（kN）" }
                                    HeadCell { "p·Lx·Ly（kN）" }
                                }
                            }
                            tbody {
                                Row {
                                    DecimalCell { value: reactions, decimals: 0 }
                                    DecimalCell { value: load, decimals: 0 }
                                }
                            }
                        }
                        Notes {
                            li { "ΣV は辺の反力の合計" }
                            li { "ΣR は隅反力の合計" }
                        }
                    }
                }
            }
        }
    }
}

/// 入力の Sheet（デザインシステムの Sheet）。対になる欄を 1 行に 2 組ずつ並べる（スマホで縦に長くしないため）。
/// 行見出し（欄の名前）は書けないので `fill`、値のセルは `paper` に `line` の格子。選んだセルは `ink`、
/// 読めないセルは `ng` の太い内枠（`outline-2`）で、行見出しも `ng`。外の角だけ `radius-8`（ツールの入力カード）。
#[component]
fn InputSheet(inputs: Signal<PlateInputs>, errors: Vec<(InputField, &'static str)>) -> Element {
    let supports = inputs.read().supports;
    let has_error = |field: InputField| errors.iter().any(|(e, _)| *e == field);
    rsx! {
        div { class: "overflow-hidden rounded-8 border border-line",
            table { class: "w-full table-fixed",
                tbody {
                    for pair in FIELD_PAIRS {
                        tr { class: "border-b border-line",
                            for field in pair {
                                SheetHead { label: field.label(), error: has_error(field) }
                                SheetCell {
                                    input {
                                        class: if has_error(field) { "h-6 w-full bg-paper px-2 font-mono text-num text-right outline-2 -outline-offset-2 outline-ng" } else { "h-6 w-full bg-paper px-2 font-mono text-num text-right focus:outline-2 focus:-outline-offset-2 focus:outline-ink" },
                                        r#type: "text",
                                        inputmode: "decimal",
                                        "aria-label": field.label(),
                                        value: inputs.read().get(field).to_string(),
                                        oninput: move |e| inputs.write().set(field, e.value()),
                                    }
                                }
                            }
                        }
                    }
                    for pair in EDGE_PAIRS {
                        tr { class: "border-b border-line last:border-b-0",
                            for (edge , direction) in pair {
                                SheetHead { label: format!("{edge}（{direction}）"), error: false }
                                SheetCell {
                                    select {
                                        class: "h-6 w-full bg-paper px-2 text-body focus:outline-2 focus:-outline-offset-2 focus:outline-ink",
                                        "aria-label": format!("{edge} の辺の支持条件"),
                                        onchange: move |e| {
                                            if let Some(support) = e.value().chars().next().and_then(Support::from_symbol) {
                                                set_support(&mut inputs.write().supports, edge, support);
                                            }
                                        },
                                        for (support , name) in SUPPORTS {
                                            option {
                                                value: "{support.symbol()}",
                                                selected: supports.at(edge) == support,
                                                {name}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Sheet の行見出し。右に `line`
#[component]
fn SheetHead(label: String, error: bool) -> Element {
    rsx! {
        th {
            class: if error { "border-r border-line bg-fill px-1 text-center text-label text-ng font-normal whitespace-nowrap" } else { "border-r border-line bg-fill px-1 text-center text-label text-muted font-normal whitespace-nowrap" },
            scope: "row",
            {label}
        }
    }
}

/// Sheet の書けるセル。右に `line`（行の最後のセルは外枠に任せる）
#[component]
fn SheetCell(children: Element) -> Element {
    rsx! {
        td { class: "border-r border-line p-0 last:border-r-0", {children} }
    }
}

/// 辺の最大値の表。行は辺、列は M・Q・V。`positions` なら値でなくその位置を並べる。単位は、値の表は
/// 列の見出しに、位置の表は表の名前に書く
#[component]
fn EdgeTable(edges: Vec<EdgeRow>, positions: bool) -> Element {
    let pick: fn(&Maximum) -> &str = if positions {
        |m| &m.along
    } else {
        |m| &m.value
    };
    let columns: [fn(&EdgeRow) -> &Maximum; 3] =
        [|r| &r.max_moment, |r| &r.max_shear, |r| &r.max_reaction];
    let decimals = columns.map(|column| max_decimals(edges.iter().map(|r| pick(column(r)))));
    let heads = if positions {
        ["M", "Q", "V"]
    } else {
        ["M（kN·m/m）", "Q（kN/m）", "V（kN/m）"]
    };
    rsx! {
        Table { fixed: true,
            thead {
                HeadRow {
                    HeadCell { left: true, "辺" }
                    for head in heads {
                        HeadCell { "{head}" }
                    }
                }
            }
            tbody {
                for row in edges.iter() {
                    Row {
                        TextCell { "{row.name}" }
                        for (column , decimals) in columns.into_iter().zip(decimals) {
                            DecimalCell { value: pick(column(row)).to_string(), decimals }
                        }
                    }
                }
            }
        }
    }
}
