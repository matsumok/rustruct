//! Tool「Kirchhoff 板の曲げ」（`/tools/kirchhoff-plate`）。言葉は `docs/tools/kirchhoff-plate/GLOSSARY.md`、
//! 符号と座標はADR kirchhoff-plate-0004。数値はすべて `rustruct-calc` の `solve` から引き、ここでは入力の文字の
//! 読み取りと、表示の桁だけを決める。計算はブラウザの中で同期に回し、入力を変えるたびに結果を描き直す。
//!
//! 辺は座標で呼ぶ（x = 0 の辺など）。画面の上下左右は図の向き（x は右、y は上）でここが決め、入力の
//! Sheet の行見出しに添えて対応を示す。
//!
//! 板の材料は入力の組のいちばん上の切り替えで選び、Sheet の E と ν の欄は材料で変わる（[`Material`]）。

use dioxus::prelude::*;
use numerus::significant;
use rustruct_calc::kirchhoff_plate::{Extreme, InvalidInput, PlateError, Solution, solve};
use rustruct_types::kirchhoff_plate::{Corner, Edge, Plate, PlateMaterial, Support, Supports};
use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};

mod figure;

use figure::corner_margin_text;
pub use figure::{PlateField, PlateFigure, Quantity};

use crate::Route;
use crate::chart;
use crate::data;
use crate::inputs::{
    Definition, Field, InputGroup, InputRecord, Inputs, Kind, Layout, Reading, use_inputs,
};
use crate::page::Page;
use crate::sheet::Switch;
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

/// 欄。E と ν は材料ごとに別の欄で、Sheet に載るのは選んだ材料のものだけ（[`rows`]）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    Lx,
    Ly,
    Thickness,
    /// 直接入力の E
    YoungModulus,
    /// 材料から導く E（コンクリートと鋼材）。読むだけ
    DerivedYoungModulus,
    /// 直接入力の ν
    PoissonRatio,
    /// コンクリートの ν
    ConcretePoissonRatio,
    /// 材料から導く ν（鋼材）。読むだけ
    DerivedPoissonRatio,
    Load,
    /// コンクリートの Fc
    Fc,
    /// 辺の支持条件
    Support(Edge),
    /// 板の材料。入力の組の上の切り替えで選び、Sheet には載らない。変えるのは [`switch_material`]
    Material,
}

impl Field for InputField {
    /// Sheet の行の順。材料は最後（警告の関わる欄の最初が Sheet に載るように）
    const ALL: &'static [Self] = &[
        Self::Lx,
        Self::Ly,
        Self::Thickness,
        Self::YoungModulus,
        Self::DerivedYoungModulus,
        Self::PoissonRatio,
        Self::ConcretePoissonRatio,
        Self::DerivedPoissonRatio,
        Self::Load,
        Self::Fc,
        Self::Support(Edge::X0),
        Self::Support(Edge::XLx),
        Self::Support(Edge::Y0),
        Self::Support(Edge::YLy),
        Self::Material,
    ];

    /// 名前と種類と初期値（Issue #36、#135）。値の規則は計算核の入口（`solve`）と、ν は `PoissonRatio`、
    /// Fc は `Concrete` が見る。開いたときは直接入力で、コンクリートの Fc と ν は初めて選んだときの値
    fn definition(self) -> Definition {
        let number = |initial| Kind::number(false, initial);
        let (name, kind) = match self {
            Self::Lx => ("Lx（m）".into(), number("4.0")),
            Self::Ly => ("Ly（m）".into(), number("5.0")),
            Self::Thickness => ("t（mm）".into(), number("150")),
            Self::YoungModulus => ("E（N/mm²）".into(), number("22700")),
            Self::DerivedYoungModulus => ("E（N/mm²）".into(), Kind::ReadOnly),
            Self::PoissonRatio => ("ν".into(), number("0")),
            Self::ConcretePoissonRatio => ("ν".into(), number("0.2")),
            Self::DerivedPoissonRatio => ("ν".into(), Kind::ReadOnly),
            Self::Load => ("p（kN/m²）".into(), number("10")),
            Self::Fc => ("Fc（N/mm²）".into(), number("24")),
            Self::Material => (
                "板の材料".into(),
                Kind::Choice {
                    options: Material::ALL
                        .map(|material| (material.symbol(), material.name()))
                        .to_vec(),
                    initial: Material::Direct.symbol(),
                },
            ),
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

/// 入力の Sheet の行。対になる欄を 1 行に 2 組ずつ（スマホで縦に長くしないため）、数値の欄の後に向かい合う
/// 辺の支持条件。E と ν の欄は材料によらず同じ場所（t の右と p の左）で、材料のものに替わる。コンクリートは
/// Fc の欄を ν・p の行の下に 1 欄だけの行で足す（空いた組は `fill` の空き）
fn rows(material: Material) -> Vec<Vec<InputField>> {
    let (young_modulus, poisson_ratio) = match material {
        Material::Concrete => (
            InputField::DerivedYoungModulus,
            InputField::ConcretePoissonRatio,
        ),
        Material::Steel => (
            InputField::DerivedYoungModulus,
            InputField::DerivedPoissonRatio,
        ),
        Material::Direct => (InputField::YoungModulus, InputField::PoissonRatio),
    };
    let mut rows = vec![
        vec![InputField::Lx, InputField::Ly],
        vec![InputField::Thickness, young_modulus],
        vec![poisson_ratio, InputField::Load],
    ];
    if material == Material::Concrete {
        rows.push(vec![InputField::Fc]);
    }
    rows.extend([
        vec![
            InputField::Support(Edge::X0),
            InputField::Support(Edge::XLx),
        ],
        vec![
            InputField::Support(Edge::Y0),
            InputField::Support(Edge::YLy),
        ],
    ]);
    rows
}

/// 板の材料の選び方。並びは切り替えの順。Model の板の材料（`PlateMaterial`）の種類で、値は欄が持つ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Concrete,
    Steel,
    Direct,
}

impl Material {
    pub const ALL: [Self; 3] = [Self::Concrete, Self::Steel, Self::Direct];

    /// 材料の欄の記号
    fn symbol(self) -> char {
        match self {
            Self::Concrete => 'C',
            Self::Steel => 'S',
            Self::Direct => 'D',
        }
    }

    /// 材料の欄の記号から。欄は選択肢の記号しか持たないので、必ず材料に戻る
    fn from_symbol(symbol: char) -> Self {
        Self::ALL
            .into_iter()
            .find(|material| material.symbol() == symbol)
            .expect("材料の欄は Material の記号")
    }

    /// 切り替えの名前
    pub fn name(self) -> &'static str {
        match self {
            Self::Concrete => "コンクリート",
            Self::Steel => "鋼材",
            Self::Direct => "直接入力",
        }
    }
}

/// 選んでいる材料
pub fn material(inputs: &PlateInputs) -> Material {
    Material::from_symbol(inputs.choice(InputField::Material))
}

/// 材料を切り替える。直接入力に切り替えると、E と ν の欄に直前の材料の E と ν を入れる（E は読むだけの欄と
/// 同じく N/mm² の整数に丸めた値）。直前の材料の値が使えない（コンクリートの Fc か ν が読めないか型が受けない）
/// ときは、その欄は前のまま。コンクリートの Fc と ν は自分の欄に残るので、コンクリートに戻すと前の値に戻る
/// （初めてなら初期値の Fc 24、ν 0.2）。値の置き場はページを開いている間だけある
pub fn switch_material(inputs: &mut PlateInputs, to: Material) {
    let from = material(inputs);
    if to == Material::Direct && from != Material::Direct {
        let mut reading = inputs.reading();
        let plate_material = plate_material(&mut reading);
        // ほかの材料の欄は読んでいないので、鋼材ではどちらも使える
        let young_modulus = (!reading.refused(InputField::Fc))
            .then(|| young_modulus_text(plate_material.young_modulus()));
        let poisson_ratio = (!reading.refused(InputField::ConcretePoissonRatio))
            .then(|| poisson_ratio_text(plate_material.poisson_ratio()));
        if let Some(text) = young_modulus {
            inputs.set(InputField::YoungModulus, text);
        }
        if let Some(text) = poisson_ratio {
            inputs.set(InputField::PoissonRatio, text);
        }
    }
    inputs.set(InputField::Material, to.symbol());
}

/// 材料から導く E の書き方。N/mm² の整数に丸める（有効数字 3 桁の表に揃えない。Fc 24 は 22669）。
/// 直接入力の欄に移すときも同じ文字
fn young_modulus_text(young_modulus: Stress) -> String {
    young_modulus.as_n_per_mm2().round().to_string()
}

/// 材料から導く ν の書き方（鋼材は 0.3）
fn poisson_ratio_text(poisson_ratio: PoissonRatio) -> String {
    poisson_ratio.value().to_string()
}

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

/// 入力を Model へ。読めない欄と、型が受けない値（ν、Fc）は理由を集めて初期値で埋める（`Reading`）
fn plate(reading: &mut Reading<'_, InputField>) -> Plate {
    Plate {
        lx: Length::m(reading.number(InputField::Lx)),
        ly: Length::m(reading.number(InputField::Ly)),
        thickness: Length::mm(reading.number(InputField::Thickness)),
        material: plate_material(reading),
        load: AreaLoad::kn_per_m2(reading.number(InputField::Load)),
        supports: supports(reading),
    }
}

/// 選んでいる材料の欄から板の材料。読むのはその材料の欄だけ。コンクリートは普通コンクリートで、Fc の規則と
/// 警告は `data::concrete`（Fc 18 未満と 100 超の警告は入力の組に出る）。読むだけの欄の値（コンクリートの E、
/// 鋼材の E と ν）もここで決める。コンクリートの E は Fc が使えるときだけ
fn plate_material(reading: &mut Reading<'_, InputField>) -> PlateMaterial {
    let poisson_ratio = |reading: &mut Reading<'_, InputField>, field| {
        reading.typed(field, PoissonRatio::new, "0 以上 0.5 未満で入れる")
    };
    match Material::from_symbol(reading.choice(InputField::Material)) {
        Material::Direct => PlateMaterial::Direct {
            young_modulus: Stress::n_per_mm2(reading.number(InputField::YoungModulus)),
            poisson_ratio: poisson_ratio(reading, InputField::PoissonRatio),
        },
        Material::Concrete => {
            let concrete = data::concrete(reading, InputField::Fc);
            let material = PlateMaterial::Concrete {
                concrete,
                poisson_ratio: poisson_ratio(reading, InputField::ConcretePoissonRatio),
            };
            if !reading.refused(InputField::Fc) {
                reading.show(
                    InputField::DerivedYoungModulus,
                    young_modulus_text(material.young_modulus()),
                );
            }
            material
        }
        Material::Steel => {
            let material = PlateMaterial::Steel;
            reading.show(
                InputField::DerivedYoungModulus,
                young_modulus_text(material.young_modulus()),
            );
            reading.show(
                InputField::DerivedPoissonRatio,
                poisson_ratio_text(material.poisson_ratio()),
            );
            material
        }
    }
}

/// 計算核の入口が返す不正な欄を、画面の欄へ。どれも欄に写る
fn field_of(reason: InvalidInput) -> Option<InputField> {
    Some(match reason {
        InvalidInput::Lx => InputField::Lx,
        InvalidInput::Ly => InputField::Ly,
        InvalidInput::Thickness => InputField::Thickness,
        InvalidInput::YoungModulus => InputField::YoungModulus,
        InvalidInput::Load => InputField::Load,
    })
}

/// 4 辺の支持条件の組。選択の欄は選択肢の記号しか持たないので、必ず支持条件に戻る
fn supports(reading: &Reading<'_, InputField>) -> Supports {
    let at = |edge| {
        Support::from_symbol(reading.choice(InputField::Support(edge)))
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
    fn not_computed() -> Self {
        Self {
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
    /// 入力の組の記録。解けない理由（不安定か、未対応か）は警告で、支持条件の欄に関わる
    pub record: InputRecord<InputField>,
    /// 結果の表
    pub readout: PlateReadout,
    /// 分布図の評価点の応答。解けなければ無い
    pub field: Option<PlateField>,
}

impl PlateOutcome {
    pub fn from_inputs(inputs: &PlateInputs) -> Self {
        let mut reading = inputs.reading();
        let solved = solve(&plate(&mut reading));
        if let Err(PlateError::InvalidInput(invalid)) = &solved {
            reading.invalid(invalid, field_of, &[]);
        }
        let supports = EDGES.map(InputField::Support);
        let solution = match solved {
            _ if reading.has_reasons() => None,
            Ok(solution) => Some(solution),
            Err(PlateError::Unstable(_)) => {
                reading.warn(&supports, "この支持条件の組では板を支えられない");
                None
            }
            // 不正な欄は上で欄に写したので来ない。来たら解けないことだけを言う
            Err(PlateError::InvalidInput(_) | PlateError::Singular) => {
                reading.warn(InputField::ALL, "この入力では解けない");
                None
            }
        };
        Self {
            record: reading.finish(),
            readout: solution
                .as_ref()
                .map_or_else(PlateReadout::not_computed, PlateReadout::computed),
            field: solution.as_ref().map(PlateField::new),
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
    let mut inputs = use_inputs::<InputField>();
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
    let chosen = material(&inputs.read());
    rsx! {
        Page { route: Route::ToolsKirchhoffPlate {},
            Columns {
                Column {
                    InputGroup {
                        inputs,
                        switch: rsx! {
                            Switch {
                                label: "板の材料",
                                names: Material::ALL.map(Material::name).to_vec(),
                                pressed: Material::ALL.iter().position(|m| *m == chosen).expect("材料は ALL に載る"),
                                symbols: false,
                                onpress: move |index: usize| switch_material(&mut inputs.write(), Material::ALL[index]),
                            }
                        },
                        layout: Layout::RowHeads(rows(chosen)),
                        record: outcome.read().record.clone(),
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

/// 分布図の一まとまり。図の名前、量の切替、図、図の注記（`PlateFigure::notes`。無ければ箇条を置かない）。
/// 支持条件は入力の表が言うので、縁の読み方は書かない。解けないときは図と注記の代わりに「—」だけを置く
/// （解けない理由は入力の下の注記が言う）
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
                        dangerous_inner_html: chart::svg(&figure.scene),
                    }
                    if !figure.notes.is_empty() {
                        Notes {
                            for note in figure.notes {
                                li { {note} }
                            }
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

/// 分布図の量の切替。5 つの量の記号を等幅で並べる（`Switch`）
#[component]
fn QuantitySwitch(quantity: Signal<Quantity>) -> Element {
    rsx! {
        Switch {
            label: "分布図の量",
            names: Quantity::ALL.map(Quantity::symbol).to_vec(),
            pressed: Quantity::ALL.iter().position(|q| *q == quantity()).expect("量は ALL に載る"),
            symbols: true,
            onpress: move |index: usize| quantity.set(Quantity::ALL[index]),
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
