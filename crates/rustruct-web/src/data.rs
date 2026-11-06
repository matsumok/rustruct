//! Data（`GLOSSARY.md`、ADR site-0005）: 材料や規格の値を、表と、値を導く式で示すページ。
//! 数値はすべて `rustruct-types` から引き、ここでは表示の桁だけを決める（Issue #19）。

use dioxus::prelude::*;
use numerus::{exponential, fixed, significant};
use rustruct_types::allowable_stress::AllowableStress;
use rustruct_types::concrete::{BarPosition, Concrete, ConcreteKind};
use rustruct_types::quantity::Stress;
use rustruct_types::rebar::{BarDesignation, RebarKind};

use crate::Route;
use crate::inputs::{Definition, Field, InputGroup, Inputs, Kind, Layout, Reason};
use crate::page::{Page, RouteList};
use crate::table::{
    Block, Caption, Cell, Column, Columns, Divider, Formula, Notes, Table, TableColumn,
};

/// ヤング係数 E の欄。有効数字 3 桁で、値に E まで書く（「2.27E+04」。ADR site-0013）。
/// 見出しは単位だけ（「E（N/mm²）」）。Data のコンクリートのページと必要スラブ厚さのページが同じ書き方を使う
pub(crate) fn young_modulus(e: Stress) -> String {
    exponential(e.as_n_per_mm2(), 3)
}

/// `/data`。Data の一覧（名前＋一行＋パス）。
#[component]
pub(crate) fn DataIndex() -> Element {
    rsx! {
        Page { route: Route::Data {},
            RouteList { routes: Route::data_pages().collect::<Vec<_>>() }
        }
    }
}

/// コンクリートのページの欄。Fc の 1 つだけ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcreteField {
    Fc,
}

impl Field for ConcreteField {
    const ALL: &'static [Self] = &[Self::Fc];

    /// 初期値 24。0 より大きいかは `Concrete` が見る
    fn definition(self) -> Definition {
        match self {
            Self::Fc => Definition {
                name: "Fc（N/mm²）".into(),
                kind: Kind::number(false, "24"),
            },
        }
    }
}

/// 入力の中身。読めるかは `ConcreteReadout` が決める
pub type ConcreteInputs = Inputs<ConcreteField>;

/// Fc の入力から求めた、カルキュレータの表示。数値は丸めて文字にしたもの。
#[derive(Debug, Clone, PartialEq)]
pub enum ConcreteReadout {
    Computed {
        /// 気乾単位体積重量 γ [kN/m³]
        unit_weight: String,
        /// ヤング係数 E。書き方は `data::young_modulus`
        young_modulus: String,
        /// 許容応力度。`Allowable::ALL` の順
        allowable_stresses: Vec<AllowableRow>,
        /// 計算はできるが、Fc が想定の範囲を外れている
        warning: Option<String>,
    },
    /// Fc が使えない。値はすべて「—」。理由は入力の組の文言（欄の名前を含めない）
    Unusable { error: Reason },
}

impl ConcreteReadout {
    /// Fc の欄に `input` を打ったときの表示
    pub fn from_fc_input(input: &str) -> Self {
        let mut inputs = ConcreteInputs::new();
        inputs.set(ConcreteField::Fc, input);
        Self::from_inputs(&inputs)
    }

    pub fn from_inputs(inputs: &ConcreteInputs) -> Self {
        match Self::concrete(inputs) {
            Ok(c) => Self::Computed {
                unit_weight: fixed(c.unit_weight().as_kn_per_m3(), 1),
                young_modulus: young_modulus(c.young_modulus()),
                allowable_stresses: Allowable::ALL
                    .map(|a| AllowableRow::new(a.name(), a.of(&c)))
                    .to_vec(),
                warning: c.warning().map(|w| w.to_string()),
            },
            Err(error) => Self::Unusable { error },
        }
    }

    /// 許容応力度の表の行（名前、長期、短期）。使えないときは値がすべて無し
    fn allowable_rows(&self) -> Vec<Vec<Cell>> {
        match self {
            Self::Computed {
                allowable_stresses, ..
            } => allowable_stresses
                .iter()
                .map(|row| {
                    vec![
                        row.name.into(),
                        row.long_term.clone().into(),
                        row.short_term.clone().into(),
                    ]
                })
                .collect(),
            Self::Unusable { .. } => Allowable::ALL
                .map(|a| vec![a.name().into(), None.into(), None.into()])
                .to_vec(),
        }
    }

    /// (γ, E) の表示。使えないときはすべて無し
    fn values(&self) -> Vec<Cell> {
        match self {
            Self::Computed {
                unit_weight,
                young_modulus,
                ..
            } => vec![unit_weight.clone().into(), young_modulus.clone().into()],
            Self::Unusable { .. } => vec![None.into(), None.into()],
        }
    }

    /// 読めない Fc は文字の規則の理由、`Concrete` が受けない Fc は「0 より大きい値を入れる」
    fn concrete(inputs: &ConcreteInputs) -> Result<Concrete, Reason> {
        let fc = inputs.number(ConcreteField::Fc)?;
        Concrete::new(Stress::n_per_mm2(fc), ConcreteKind::Normal)
            .map_err(|_| "0 より大きい値を入れる".into())
    }

    /// 入力の組に渡す、読めない欄と理由
    fn errors(&self) -> Vec<(ConcreteField, Reason)> {
        match self {
            Self::Computed { .. } => Vec::new(),
            Self::Unusable { error } => vec![(ConcreteField::Fc, error.clone())],
        }
    }

    fn warning(&self) -> Option<String> {
        match self {
            Self::Computed { warning, .. } => warning.clone(),
            Self::Unusable { .. } => None,
        }
    }
}

/// コンクリートの許容応力度の表の行。名前・長期の式・値の引き方を行ごとに持ち、結果と根拠の表は `ALL` の順に並ぶ
#[derive(Debug, Clone, Copy)]
enum Allowable {
    Compression,
    Shear,
    BondTop,
    BondOther,
}

impl Allowable {
    const ALL: [Self; 4] = [
        Self::Compression,
        Self::Shear,
        Self::BondTop,
        Self::BondOther,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Compression => "圧縮",
            Self::Shear => "せん断",
            Self::BondTop => "付着（上端筋）",
            Self::BondOther => "付着（その他の鉄筋）",
        }
    }

    /// 根拠の表の名前。式が2行になる付着は、名前も括弧書きの前で改行する。1行では式と短期の列に押されて
    /// スマホ幅で「（その他／の鉄筋）」と語の途中で切れたため
    fn basis_name(self) -> String {
        self.name().replacen('（', "\n（", 1)
    }

    /// 長期の根拠の式。規準の「A かつ B 以下」は min(A, B) と書く
    fn long_term_formula(self) -> Formula {
        match self {
            Self::Compression => Formula::Plain("Fc/3"),
            Self::Shear => Formula::Min("Fc/30", "0.49+Fc/100"),
            Self::BondTop => Formula::Min("Fc/15", "0.9+2Fc/75"),
            Self::BondOther => Formula::Min("Fc/10", "1.35+Fc/25"),
        }
    }

    /// 短期の根拠。規準の「長期に対する値の n 倍」
    fn short_term_formula(self) -> &'static str {
        match self {
            Self::Compression => "長期 × 2",
            Self::Shear | Self::BondTop | Self::BondOther => "長期 × 1.5",
        }
    }

    fn of(self, c: &Concrete) -> AllowableStress {
        match self {
            Self::Compression => c.allowable_compression(),
            Self::Shear => c.allowable_shear(),
            Self::BondTop => c.allowable_bond(BarPosition::Top),
            Self::BondOther => c.allowable_bond(BarPosition::Other),
        }
    }
}

/// 許容応力度の表の1行。値は有効数字 3 桁の文字（ADR materials-0003）。
#[derive(Debug, Clone, PartialEq)]
pub struct AllowableRow {
    pub name: &'static str,
    pub long_term: String,
    pub short_term: String,
}

impl AllowableRow {
    fn new(name: &'static str, stress: AllowableStress) -> Self {
        Self {
            name,
            long_term: significant(stress.long_term.as_n_per_mm2(), 3),
            short_term: significant(stress.short_term.as_n_per_mm2(), 3),
        }
    }
}

/// 呼び名の表の行: 呼び名, d, l, S, w。JIS G 3112 表6 と同じ桁
fn designation_rows() -> Vec<Vec<Cell>> {
    BarDesignation::ALL
        .iter()
        .map(|designation| {
            vec![
                designation.name().into(),
                significant(designation.nominal_diameter().as_mm(), 3).into(),
                fixed(designation.nominal_perimeter().as_mm(), 1).into(),
                significant(designation.nominal_area().as_mm2(), 4).into(),
                significant(designation.unit_mass().as_kg_per_m(), 3).into(),
            ]
        })
        .collect()
}

/// 気乾単位体積重量の表の行: Fc の区分, γ
fn unit_weight_rows() -> Vec<Vec<Cell>> {
    let mut lower: Option<String> = None;
    ConcreteKind::Normal
        .unit_weight_table()
        .iter()
        .map(|row| {
            let upper = integer(row.fc_upper);
            let range = match &lower {
                None => format!("Fc ≦ {upper}"),
                Some(lower) => format!("{lower} < Fc ≦ {upper}"),
            };
            lower = Some(upper);
            vec![
                range.into(),
                fixed(row.unit_weight.as_kn_per_m3(), 1).into(),
            ]
        })
        .collect()
}

/// `/data/concrete`。入力（Fc）、結果（γ・γ₁・E・許容応力度）、根拠（表と式）の順に分けて置く。
/// 結果は入力から出た値、根拠は入力に依らない規準の表と式だと、部の見出しで分かるようにする。
#[component]
pub(crate) fn ConcretePage() -> Element {
    let inputs = use_signal(ConcreteInputs::default);
    let readout = ConcreteReadout::from_inputs(&inputs.read());
    let values = readout.values();
    let allowable_rows = readout.allowable_rows();
    let fc_max = integer(
        ConcreteKind::Normal
            .unit_weight_table()
            .last()
            .expect("表の区分は1つ以上")
            .fc_upper,
    );
    rsx! {
        Page { route: Route::DataConcrete {},
            Columns {
                Column {
                    // 入力の組のすぐ下に結果（`space-6`）。この組と次の表の間は Column の間（`space-7`）
                    InputGroup {
                        inputs,
                        layout: Layout::RowHeads(vec![vec![ConcreteField::Fc]]),
                        errors: readout.errors(),
                        warning: readout.warning(),
                        Table {
                            columns: vec![
                                TableColumn::number("気乾単位体積重量 γ").unit("kN/m³"),
                                TableColumn::number("ヤング係数 E").unit("N/mm²"),
                            ],
                            rows: vec![values],
                        }
                    }
                    Block {
                        Caption { "許容応力度（N/mm²）" }
                        Table {
                            columns: vec![
                                TableColumn::name(""),
                                TableColumn::number("長期"),
                                TableColumn::number("短期"),
                            ],
                            rows: allowable_rows,
                        }
                    }
                }
                Divider {}
                Column { id: "basis",
                    Block {
                        Caption { "気乾単位体積重量" }
                        Table {
                            columns: vec![
                                TableColumn::name("Fc").unit("N/mm²"),
                                TableColumn::number("γ").unit("kN/m³"),
                            ],
                            rows: unit_weight_rows(),
                        }
                        Notes {
                            li { "Fc {fc_max} を超えると最後の区分を使う" }
                        }
                        // 出典: 実務の慣例の値（画面には出さない。ADR に書いてある）
                    }
                    Block {
                        Caption { "ヤング係数（N/mm²）" }
                        p { class: "font-mono text-num",
                            "E = 3.35×10⁴ × (γ/24)² × (Fc/60)"
                            sup { "1/3" }
                        }
                        // 出典: RC 規準（2024 年版）（画面には出さない。ADR に書いてある）
                    }
                    Block {
                        Caption { "許容応力度（N/mm²）" }
                        Table {
                            columns: vec![
                                TableColumn::name(""),
                                TableColumn::formula("長期"),
                                TableColumn::formula("短期"),
                            ],
                            rows: Allowable::ALL
                                .map(|a| {
                                    vec![
                                        a.basis_name().into(),
                                        a.long_term_formula().into(),
                                        Formula::Plain(a.short_term_formula()).into(),
                                    ]
                                })
                                .to_vec(),
                        }
                        // 出典: RC 規準（2024 年版）表 6.1・6.3（画面には出さない。ADR に書いてある）
                    }
                }
            }
        }
    }
}

/// `/data/rebar`。呼び名の表と、種類ごとの許容応力度の表。値は JIS と RC 規準の表と同じ桁で出す。
#[component]
pub(crate) fn RebarPage() -> Element {
    rsx! {
        Page { route: Route::DataRebar {},
            Columns {
                Column {
                    Block {
                        Caption { "異形棒鋼の寸法、単位質量及び節の許容限度" }
                        Table {
                            columns: vec![
                                TableColumn::name("呼び名"),
                                TableColumn::number("公称直径\nd").unit("mm"),
                                TableColumn::number("公称周長\nl").unit("mm"),
                                TableColumn::number("公称断面積\nS").unit("mm²"),
                                TableColumn::number("単位質量\nw").unit("kg/m"),
                            ],
                            rows: designation_rows(),
                        }
                        Notes {
                            li { "l = 3.142d、S = 0.7854d²、w = 0.00785S" }
                            li { "JIS Z 8401 の規則 A で丸める" }
                        }
                        // 出典: JIS G 3112:2025 表6（画面には出さない。ADR に書いてある）
                    }
                }
                Column {
                    Block {
                        Caption { "許容応力度（N/mm²）" }
                        Table {
                            columns: vec![
                                TableColumn::name("種類"),
                                TableColumn::number("引張・圧縮").group("長期"),
                                TableColumn::paren(),
                                TableColumn::number("せん断\n補強").group("長期"),
                                TableColumn::number("引張・\n圧縮").group("短期"),
                                TableColumn::number("せん断\n補強").group("短期"),
                            ],
                            rows: RebarKind::ALL
                                .iter()
                                .map(|&kind| {
                                    vec![
                                        kind.symbol().into(),
                                        long_term_tension_compression(kind).into(),
                                        long_term_tension_compression_d29_and_up(kind).into(),
                                        integer(kind.allowable_shear_reinforcement().long_term).into(),
                                        integer(kind.allowable_tension_compression(BarDesignation::D25).short_term)
                                            .into(),
                                        integer(kind.allowable_shear_reinforcement().short_term).into(),
                                    ]
                                })
                                .collect(),
                        }
                        Notes {
                            li { "* D29 以上は（ ）内の値" }
                        }
                        // 出典: RC 規準（2024 年版）表 6.2（画面には出さない。ADR に書いてある）
                    }
                }
            }
        }
    }
}

/// 長期の引張・圧縮（D29 より細い鉄筋）。D29 以上で値が変わる種類は、表 6.2 と同じく「215（*195）」と
/// 書く。括弧書きは隣のセル（`long_term_tension_compression_d29_and_up`）、凡例は注記に置く
fn long_term_tension_compression(kind: RebarKind) -> String {
    integer(
        kind.allowable_tension_compression(BarDesignation::D25)
            .long_term,
    )
}

/// 長期の引張・圧縮の D29 以上の値の括弧書き「（*195）」。細い鉄筋と同じ値なら空
fn long_term_tension_compression_d29_and_up(kind: RebarKind) -> String {
    let d29_and_up = integer(
        kind.allowable_tension_compression(BarDesignation::D29)
            .long_term,
    );
    if d29_and_up == long_term_tension_compression(kind) {
        String::new()
    } else {
        format!("（*{d29_and_up}）")
    }
}

/// 表に整数で書かれている値（表 6.2 の許容応力度、Fc の区分の境）は、そのまま整数で出す
fn integer(stress: Stress) -> String {
    fixed(stress.as_n_per_mm2(), 0)
}
