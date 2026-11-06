//! Tool「Kirchhoff 板の曲げ」（`/tools/kirchhoff-plate`）。言葉は `docs/tools/kirchhoff-plate/GLOSSARY.md`、
//! 符号と座標はADR kirchhoff-plate-0004。数値はすべて `rustruct-calc` の `solve` から引き、ここでは入力の文字の
//! 読み取りと、表示の桁だけを決める。計算はブラウザの中で同期に回し、入力を変えるたびに結果を描き直す。
//!
//! 辺は座標で呼ぶ（x = 0 の辺など）。画面の上下左右は図の向き（x は右、y は上）でここが決め、入力の
//! Sheet の行見出しに添えて対応を示す。

use dioxus::prelude::*;
use numerus::significant;
use rustruct_calc::kirchhoff_plate::{Extreme, InvalidInput, PlateError, Solution, solve};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Plate, Support, Supports};
use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};

mod figure;

use figure::corner_margin_text;
pub use figure::{PlateField, PlateFigure, Quantity};

use crate::Route;
use crate::inputs::{Definition, Field, InputGroup, Inputs, Kind, Layout, Reading, Reason};
use crate::page::Page;
use crate::table::{Block, Caption, Column, Columns, NO_VALUE, Notes, Table, TableColumn};

/// 辺の最大値を取る評価点の分け方。辺を等分する数
const EDGE_DIVISIONS: usize = 100;

/// 自由辺のある組の、辺の最大値の注記（ADR kirchhoff-plate-0008）
fn corner_notes() -> Vec<String> {
    vec![
        "固定辺と自由辺の隅では、せん断力と辺の反力が発散する".into(),
        format!(
            "辺のせん断力と反力の最大値は、隅から辺長の {} を除いた範囲で取る",
            corner_margin_text()
        ),
    ]
}

/// 欄。並びは Sheet の行の順
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    Lx,
    Ly,
    Thickness,
    YoungModulus,
    PoissonRatio,
    Load,
    /// 辺の支持条件
    Support(Edge),
}

impl Field for InputField {
    const ALL: &'static [Self] = ROWS.as_flattened();

    /// 名前と種類と初期値（Issue #36）。値の規則は計算核の入口（`solve`）と、ν は `PoissonRatio` が見る
    fn definition(self) -> Definition {
        let number = |initial| Kind::number(false, initial);
        let (name, kind) = match self {
            Self::Lx => ("Lx（m）".into(), number("4.0")),
            Self::Ly => ("Ly（m）".into(), number("5.0")),
            Self::Thickness => ("t（mm）".into(), number("150")),
            Self::YoungModulus => ("E（N/mm²）".into(), number("22700")),
            Self::PoissonRatio => ("ν".into(), number("0")),
            Self::Load => ("p（kN/m²）".into(), number("10")),
            Self::Support(edge) => (
                format!("{edge}（{}）", direction(edge)),
                Kind::Choice {
                    options: SUPPORTS
                        .map(|support| (support.symbol(), support_name(support)))
                        .to_vec(),
                    initial: Support::Fixed.symbol(),
                },
            ),
        };
        Definition { name, kind }
    }
}

/// 辺の並び。入力の Sheet と結果の表の行の順（左右、上下）
const EDGES: [Edge; 4] = [Edge::X0, Edge::XLx, Edge::Y0, Edge::YLy];

/// 辺の画面での呼び方。図の向き（x は右、y は上）で決める
fn direction(edge: Edge) -> &'static str {
    match edge {
        Edge::X0 => "左",
        Edge::XLx => "右",
        Edge::Y0 => "下",
        Edge::YLy => "上",
    }
}

/// 入力の Sheet の行。対になる欄を 1 行に 2 組ずつ（スマホで縦に長くしないため）、
/// 数値の欄の後に向かい合う辺の支持条件
const ROWS: [[InputField; 2]; 5] = [
    [InputField::Lx, InputField::Ly],
    [InputField::Thickness, InputField::YoungModulus],
    [InputField::PoissonRatio, InputField::Load],
    [
        InputField::Support(Edge::X0),
        InputField::Support(Edge::XLx),
    ],
    [
        InputField::Support(Edge::Y0),
        InputField::Support(Edge::YLy),
    ],
];

/// 支持条件の選び方。並びは選択肢の順
const SUPPORTS: [Support; 3] = [Support::Fixed, Support::SimplySupported, Support::Free];

/// 支持条件の画面での名前
fn support_name(support: Support) -> &'static str {
    match support {
        Support::Fixed => "固定",
        Support::SimplySupported => "単純支持",
        Support::Free => "自由",
    }
}

/// 入力の Sheet の中身。読めるかは `PlateOutcome` が決める
pub type PlateInputs = Inputs<InputField>;

/// 入力を Model へ。読めない欄と、`PoissonRatio` が受けない ν は理由を集めて初期値で埋める（`Reading`）
fn plate(inputs: &PlateInputs, reading: &mut Reading<'_, InputField>) -> Plate {
    let nu = reading.number(InputField::PoissonRatio);
    let poisson_ratio = PoissonRatio::new(nu).unwrap_or_else(|_| {
        reading.refuse(InputField::PoissonRatio, "0 以上 0.5 未満で入れる");
        let initial = reading.initial(InputField::PoissonRatio);
        PoissonRatio::new(initial.expect("ν は空欄を受けない"))
            .expect("ν の初期値は 0 以上 0.5 未満")
    });
    Plate {
        lx: Length::m(reading.number(InputField::Lx)),
        ly: Length::m(reading.number(InputField::Ly)),
        thickness: Length::mm(reading.number(InputField::Thickness)),
        young_modulus: Stress::n_per_mm2(reading.number(InputField::YoungModulus)),
        poisson_ratio,
        load: AreaLoad::kn_per_m2(reading.number(InputField::Load)),
        supports: supports(inputs),
    }
}

/// 計算核の入口が返す不正な欄を、画面の欄へ
fn field_of(reason: InvalidInput) -> InputField {
    match reason {
        InvalidInput::Lx => InputField::Lx,
        InvalidInput::Ly => InputField::Ly,
        InvalidInput::Thickness => InputField::Thickness,
        InvalidInput::YoungModulus => InputField::YoungModulus,
        InvalidInput::Load => InputField::Load,
    }
}

/// 4 辺の支持条件の組。選択の欄は選択肢の記号しか持たないので、必ず支持条件に戻る
fn supports(inputs: &PlateInputs) -> Supports {
    let at = |edge| {
        Support::from_symbol(inputs.choice(InputField::Support(edge)))
            .expect("選択肢は SUPPORTS の記号")
    };
    Supports {
        x0: at(Edge::X0),
        y0: at(Edge::Y0),
        x_lx: at(Edge::XLx),
        y_ly: at(Edge::YLy),
    }
}

/// 入力から求めた、結果の表の表示。数値は丸めて文字にしたもの。計算できないときは無し（「—」）
#[derive(Debug, Clone, PartialEq)]
pub struct PlateReadout {
    /// 読めない欄と、その理由
    pub errors: Vec<(InputField, Reason)>,
    /// 支持条件の組を解けない理由（不安定か、未対応か）。「!」を付けて出す
    pub notice: Option<&'static str>,
    /// 板の中央の値
    pub center: CenterRow,
    /// 最大たわみ（ADR kirchhoff-plate-0009）
    pub deflection_maximum: DeflectionRow,
    /// 辺ごとの行。`EDGES` の順
    pub edges: Vec<EdgeRow>,
    /// 辺の最大値の表に足す注記。自由辺のある組だけ（ADR kirchhoff-plate-0008）
    pub edge_notes: Vec<String>,
    /// 隅ごとの隅反力（kN）。`Corner::ALL` の順
    pub corners: Vec<(String, Option<String>)>,
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
    pub w: Option<String>,
    /// 曲げモーメント（kN·m/m）
    pub mx: Option<String>,
    pub my: Option<String>,
}

/// 結果の表の、最大たわみの行
#[derive(Debug, Clone, PartialEq)]
pub struct DeflectionRow {
    /// たわみ（mm）
    pub w: Option<String>,
    /// 位置（m）。隅 x = 0・y = 0 からの座標
    pub x: Option<String>,
    pub y: Option<String>,
}

/// 辺の最大値の 1 つ
#[derive(Debug, Clone, PartialEq)]
pub struct Maximum {
    /// 絶対値が最大の値（曲げモーメントは kN·m/m、せん断力と辺の反力は kN/m）
    pub value: Option<String>,
    /// 辺の始点からの位置（m）。値が辺の全体で 0 のときは無し
    pub along: Option<String>,
}

impl Maximum {
    /// 計算核の辺の最大値から。`value` は表の単位の値
    fn of<T: Copy>(extreme: Extreme<T>, value: fn(T) -> f64) -> Self {
        Self::new(value(extreme.value), extreme.along)
    }

    fn new(value: f64, along: Length) -> Self {
        Self {
            value: figures(value),
            along: if value == 0.0 {
                None
            } else {
                figures(along.as_m())
            },
        }
    }

    fn not_computed() -> Self {
        Self {
            value: None,
            along: None,
        }
    }
}

/// 釣り合いの行（kN）
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    /// 辺の反力の合計 − 隅反力の合計
    pub reactions: Option<String>,
    /// 荷重の合計
    pub load: Option<String>,
}

impl PlateReadout {
    fn not_computed(notice: Option<&'static str>) -> Self {
        Self {
            errors: Vec::new(),
            notice,
            center: CenterRow {
                w: None,
                mx: None,
                my: None,
            },
            deflection_maximum: DeflectionRow {
                w: None,
                x: None,
                y: None,
            },
            edges: EDGES
                .map(|edge| EdgeRow {
                    name: edge.to_string(),
                    max_moment: Maximum::not_computed(),
                    max_shear: Maximum::not_computed(),
                    max_reaction: Maximum::not_computed(),
                })
                .to_vec(),
            edge_notes: Vec::new(),
            corners: Corner::ALL
                .map(|corner| (corner_name(corner), None))
                .to_vec(),
            balance: Balance {
                reactions: None,
                load: None,
            },
        }
    }

    fn computed(solution: &Solution) -> Self {
        let plate = solution.plate();
        let center = solution
            .at(
                Length::mm(plate.lx.as_mm() / 2.0),
                Length::mm(plate.ly.as_mm() / 2.0),
            )
            .expect("中央は板の上");
        let deflection = solution.deflection_maximum();
        let edges = EDGES
            .map(|edge| {
                let maximum = solution.edge_maximum::<EDGE_DIVISIONS>(edge);
                EdgeRow {
                    name: edge.to_string(),
                    max_moment: Maximum::of(maximum.moment, |m| m.as_knm_per_m()),
                    max_shear: Maximum::of(maximum.shear, |q| q.as_kn_per_m()),
                    max_reaction: Maximum::of(maximum.reaction, |v| v.as_kn_per_m()),
                }
            })
            .to_vec();
        Self {
            errors: Vec::new(),
            notice: None,
            center: CenterRow {
                w: figures(center.w.as_mm()),
                mx: figures(center.mx.as_knm_per_m()),
                my: figures(center.my.as_knm_per_m()),
            },
            deflection_maximum: DeflectionRow {
                w: figures(deflection.w.as_mm()),
                x: figures(deflection.x.as_m()),
                y: figures(deflection.y.as_m()),
            },
            edges,
            edge_notes: if plate.supports.has_free_edge() {
                corner_notes()
            } else {
                Vec::new()
            },
            corners: Corner::ALL
                .map(|corner| {
                    (
                        corner_name(corner),
                        figures(solution.corner_reaction(corner).as_kn()),
                    )
                })
                .to_vec(),
            balance: Balance {
                reactions: figures(solution.net_reaction().as_kn()),
                load: figures(solution.total_load().as_kn()),
            },
        }
    }
}

/// 計算の結果。入力を 1 回読んで解き、結果の表と分布図の素にしたもの。この Tool の入口はここだけ
#[derive(Debug, Clone, PartialEq)]
pub struct PlateOutcome {
    /// 結果の表。読めない欄と、解けない理由も
    pub readout: PlateReadout,
    /// 分布図の評価点の応答。解けなければ無い
    pub field: Option<PlateField>,
}

impl PlateOutcome {
    pub fn from_inputs(inputs: &PlateInputs) -> Self {
        let not_computed = |notice| Self {
            readout: PlateReadout::not_computed(Some(notice)),
            field: None,
        };
        let mut reading = inputs.reading();
        let solved = solve(&plate(inputs, &mut reading));
        if let Err(PlateError::InvalidInput(invalid)) = &solved {
            for &reason in invalid {
                reading.refuse(field_of(reason), reason.to_string());
            }
        }
        let errors = reading.errors();
        if !errors.is_empty() {
            return Self {
                readout: PlateReadout {
                    errors,
                    ..PlateReadout::not_computed(None)
                },
                field: None,
            };
        }
        match solved {
            Ok(solution) => Self {
                readout: PlateReadout::computed(&solution),
                field: Some(PlateField::new(&solution)),
            },
            Err(PlateError::Unstable(_)) => not_computed("この支持条件の組では板を支えられない"),
            // 不正な欄は上で欄に写したので来ない。来たら解けないことだけを言う
            Err(PlateError::InvalidInput(_) | PlateError::Singular) => {
                not_computed("この入力では解けない")
            }
        }
    }
}

/// 隅の名前。会う 2 辺で呼ぶ（x = 0・y = 0）
fn corner_name(corner: Corner) -> String {
    let (x, y) = corner.edges();
    format!("{x}・{y}")
}

/// 有効数字 3 桁（Data と同じ）。表の値なので「値か無し」の値
fn figures(value: f64) -> Option<String> {
    Some(significant(value, 3))
}

/// `/tools/kirchhoff-plate`。入力の Sheet のすぐ下に結果の表を続けた列と、分布図の列（ADR site-0006）。
/// 「入力」「結果」の部の見出しは置かず、表の見出しで区切る。解くのは入力が変わったときだけで、
/// 分布図の量を切り替えても解き直さない。
#[component]
pub(crate) fn KirchhoffPlatePage() -> Element {
    let inputs = use_signal(PlateInputs::default);
    let quantity = use_signal(|| Quantity::W);
    let outcome = use_memo(move || PlateOutcome::from_inputs(&inputs.read()));
    let figure = use_memo(move || {
        outcome
            .read()
            .field
            .as_ref()
            .map(|field| PlateFigure::new(field, quantity()))
    });
    let readout = outcome.read().readout.clone();
    let CenterRow { w, mx, my } = readout.center.clone();
    let deflection = readout.deflection_maximum.clone();
    let Balance { reactions, load } = readout.balance.clone();
    rsx! {
        Page { route: Route::ToolsKirchhoffPlate {},
            Columns {
                Column {
                    InputGroup {
                        inputs,
                        layout: Layout::RowHeads(ROWS.map(|row| row.to_vec()).to_vec()),
                        notes: vec!["ν は 0 を基本とする".into()],
                        errors: readout.errors.clone(),
                        warning: readout.notice.map(String::from),
                        Block {
                            Caption { "板の中央" }
                            Table {
                                fixed: true,
                                columns: vec![
                                    TableColumn::number("w").unit("mm"),
                                    TableColumn::number("Mx").unit("kN·m/m"),
                                    TableColumn::number("My").unit("kN·m/m"),
                                ],
                                rows: vec![vec![w.into(), mx.into(), my.into()]],
                            }
                            Notes {
                                li { "たわみは下向きを正とする" }
                                li { "曲げモーメントは下側が引張になる向きを正とする" }
                            }
                        }
                    }
                    Block {
                        Caption { "最大たわみ" }
                        Table {
                            fixed: true,
                            columns: vec![
                                TableColumn::number("w").unit("mm"),
                                TableColumn::number("x").unit("m"),
                                TableColumn::number("y").unit("m"),
                            ],
                            rows: vec![vec![deflection.w.into(), deflection.x.into(), deflection.y.into()]],
                        }
                        Notes {
                            li { "板全体での絶対値の最大値" }
                            li { "x、y は隅 x = 0・y = 0 からの座標" }
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
                            for note in readout.edge_notes.iter().cloned() {
                                li { {note} }
                            }
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
                        Table {
                            fixed: true,
                            columns: vec![TableColumn::name("隅"), TableColumn::number("R").unit("kN")],
                            rows: readout
                                .corners
                                .iter()
                                .cloned()
                                .map(|(name, reaction)| vec![name.into(), reaction.into()])
                                .collect(),
                        }
                        Notes {
                            li { "隅反力は板を押さえる向きを正とする" }
                        }
                    }
                    Block {
                        Caption { "釣り合い" }
                        Table {
                            fixed: true,
                            columns: vec![
                                TableColumn::number("ΣV − ΣR").unit("kN"),
                                TableColumn::number("p·Lx·Ly").unit("kN"),
                            ],
                            rows: vec![vec![reactions.into(), load.into()]],
                        }
                        Notes {
                            li { "ΣV は辺の反力の合計" }
                            li { "ΣR は隅反力の合計" }
                        }
                    }
                }
                Column {
                    FigureBlock { quantity, figure: figure() }
                }
            }
        }
    }
}

/// 分布図の一まとまり。図の名前、量の切替、図、支持条件の読み方の注記。解けないときは図と読み方の
/// 代わりに「—」だけを置く（解けない理由は入力の下の注記が言う）
#[component]
fn FigureBlock(quantity: Signal<Quantity>, figure: Option<PlateFigure>) -> Element {
    rsx! {
        Block {
            Caption { {quantity().caption()} }
            QuantitySwitch { quantity }
            match figure {
                Some(figure) => rsx! {
                    figure {
                        role: "img",
                        "aria-label": figure.label,
                        dangerous_inner_html: figure.svg,
                    }
                    Notes {
                        li { "斜線の付いた辺は固定" }
                        li { "線だけの辺は単純支持" }
                        li { "線の無い辺は自由" }
                        for note in figure.notes {
                            li { {note} }
                        }
                    }
                },
                None => rsx! {
                    p { {NO_VALUE} }
                },
            }
        }
    }
}

/// 分布図の量の切替。5 つのボタンを 1 行に並べ、押されているものを `ink` で塗る（選択は墨）。
/// 器は入力と同じ `line` の枠で、外の角だけ `radius-8`（ボタン）。振る舞いは `dioxus-primitives` の
/// toggle_group（`aria-pressed`、Tab では組に 1 度だけ止まり、組の中は左右の矢印と Home・End で移る）。
/// 押されているものをもう一度押しても外さない（いつも 1 つが押されている）。キーボードのフォーカスは
/// `ink` の内枠（押されて `ink` で塗ったものは `paper`）。外に描く既定の輪は器の `overflow-hidden` に切られ、
/// 塗ったものの上では見えない
#[component]
fn QuantitySwitch(quantity: Signal<Quantity>) -> Element {
    use dioxus_primitives::toggle_group::{ToggleGroup, ToggleItem};
    use std::collections::HashSet;

    let pressed = Quantity::ALL.iter().position(|q| *q == quantity());
    rsx! {
        ToggleGroup {
            class: "flex overflow-hidden rounded-8 border border-line",
            role: "group",
            aria_label: "分布図の量",
            horizontal: true,
            pressed: Some(pressed.into_iter().collect::<HashSet<usize>>()),
            on_pressed_change: move |pressed: HashSet<usize>| {
                if let Some(index) = pressed.into_iter().next() {
                    quantity.set(Quantity::ALL[index]);
                }
            },
            for (index , q) in Quantity::ALL.iter().copied().enumerate() {
                ToggleItem {
                    class: "h-6 flex-1 border-r border-line bg-paper font-mono text-num last:border-r-0 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ink aria-pressed:bg-ink aria-pressed:text-paper aria-pressed:focus-visible:outline-paper",
                    index,
                    {q.symbol()}
                }
            }
        }
    }
}

/// 辺の最大値の表。行は辺、列は M・Q・V。`positions` なら値でなくその位置を並べる。単位は、値の表は
/// 列の見出しに、位置の表は表の名前に書く
#[component]
fn EdgeTable(edges: Vec<EdgeRow>, positions: bool) -> Element {
    let pick = |m: &Maximum| {
        if positions {
            m.along.clone()
        } else {
            m.value.clone()
        }
    };
    let heads = [("M", "kN·m/m"), ("Q", "kN/m"), ("V", "kN/m")].map(|(head, unit)| {
        let column = TableColumn::number(head);
        if positions { column } else { column.unit(unit) }
    });
    let columns = [TableColumn::name("辺")].into_iter().chain(heads).collect();
    let rows = edges
        .iter()
        .map(|row| {
            vec![
                row.name.clone().into(),
                pick(&row.max_moment).into(),
                pick(&row.max_shear).into(),
                pick(&row.max_reaction).into(),
            ]
        })
        .collect();
    rsx! {
        Table { fixed: true, columns, rows }
    }
}
