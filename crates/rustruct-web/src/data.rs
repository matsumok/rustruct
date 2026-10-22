//! Data（`GLOSSARY.md`、`docs/adr/0005`）: 材料や規格の値を、表と、値を導く式で示すページ。
//! 数値はすべて `rustruct-types` から引き、ここでは表示の桁だけを決める（Issue #19）。

use dioxus::prelude::*;
use rustruct_types::allowable_stress::AllowableStress;
use rustruct_types::concrete::{BarPosition, Concrete, ConcreteKind};
use rustruct_types::quantity::Stress;
use rustruct_types::rebar::{BarDesignation, RebarKind};

use crate::table::{
    Block, Caption, Column, Columns, DecimalCell, Divider, Formula, FormulaCell, GroupCell,
    GroupRow, HeadCell, HeadRow, Notes, NumCell, ParenCell, Row, Table, TextCell, column_decimals,
    max_decimals, significant, thousands,
};
use crate::{Page, Route, RouteList};

/// 計算できない値の表示（`docs/ui-decisions.md`「エラーと警告」）
const NOT_COMPUTED: &str = "—";

/// `/data`。Data の一覧（名前＋一行＋パス）。
#[component]
pub(crate) fn DataIndex() -> Element {
    rsx! {
        Page { route: Route::Data {},
            RouteList { routes: Route::data_pages().collect::<Vec<_>>() }
        }
    }
}

/// Fc の入力から求めた、カルキュレータの表示。数値は丸めて文字にしたもの。
#[derive(Debug, Clone, PartialEq)]
pub enum ConcreteReadout {
    Computed {
        /// 気乾単位体積重量 γ [kN/m³]
        unit_weight: String,
        /// ヤング係数 E [N/mm²]。×10⁴ で有効数字 3 桁
        young_modulus: String,
        /// 許容応力度。`Allowable::ALL` の順
        allowable_stresses: Vec<AllowableRow>,
        /// 計算はできるが、Fc が想定の範囲を外れている
        warning: Option<String>,
    },
    /// Fc が使えない。値はすべて「—」
    Unusable { error: String },
}

impl ConcreteReadout {
    pub fn from_fc_input(input: &str) -> Self {
        match Self::concrete(input) {
            Ok(c) => Self::Computed {
                unit_weight: format!("{:.1}", c.unit_weight().as_kn_per_m3()),
                young_modulus: format!(
                    "{}×10⁴",
                    significant(c.young_modulus().as_n_per_mm2() / 1e4, 3)
                ),
                allowable_stresses: Allowable::ALL
                    .map(|a| AllowableRow::new(a.name(), a.of(&c)))
                    .to_vec(),
                warning: c.warning().map(|w| w.to_string()),
            },
            Err(error) => Self::Unusable { error },
        }
    }

    /// 許容応力度の表の行。使えないときは値がすべて「—」
    fn allowable_rows(&self) -> Vec<AllowableRow> {
        match self {
            Self::Computed {
                allowable_stresses, ..
            } => allowable_stresses.clone(),
            Self::Unusable { .. } => Allowable::ALL
                .map(|a| AllowableRow {
                    name: a.name(),
                    long_term: NOT_COMPUTED.into(),
                    short_term: NOT_COMPUTED.into(),
                })
                .to_vec(),
        }
    }

    /// (γ, E) の表示。使えないときはすべて「—」
    fn values(&self) -> [&str; 2] {
        match self {
            Self::Computed {
                unit_weight,
                young_modulus,
                ..
            } => [unit_weight, young_modulus],
            Self::Unusable { .. } => [NOT_COMPUTED; 2],
        }
    }

    fn concrete(input: &str) -> Result<Concrete, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("Fc を入れる".into());
        }
        let fc: f64 = input.parse().map_err(|_| "数値で入れる".to_string())?;
        Concrete::new(Stress::n_per_mm2(fc), ConcreteKind::Normal).map_err(|e| e.to_string())
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

/// 許容応力度の表の1行。値は有効数字 3 桁の文字（`docs/materials/adr/0003`）。
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

/// 呼び名の表の行: (呼び名, [d, l, S, w])。JIS G 3112 表6 と同じ桁
fn designation_rows() -> Vec<(&'static str, [String; 4])> {
    BarDesignation::ALL
        .iter()
        .map(|designation| {
            (
                designation.name(),
                [
                    significant(designation.nominal_diameter().as_mm(), 3),
                    format!("{:.1}", designation.nominal_perimeter().as_mm()),
                    thousands(significant(designation.nominal_area().as_mm2(), 4)),
                    significant(designation.unit_mass().as_kg_per_m(), 3),
                ],
            )
        })
        .collect()
}

/// 気乾単位体積重量の表の行: (Fc の区分, γ)
fn unit_weight_rows() -> Vec<(String, String)> {
    let mut lower: Option<f64> = None;
    ConcreteKind::Normal
        .unit_weight_table()
        .iter()
        .map(|row| {
            let upper = row.fc_upper.as_n_per_mm2();
            let range = match lower {
                None => format!("Fc ≦ {upper}"),
                Some(lower) => format!("{lower} < Fc ≦ {upper}"),
            };
            lower = Some(upper);
            (range, format!("{:.1}", row.unit_weight.as_kn_per_m3()))
        })
        .collect()
}

/// `/data/concrete`。入力（Fc）、結果（γ・γ₁・E・許容応力度）、根拠（表と式）の順に分けて置く。
/// 結果は入力から出た値、根拠は入力に依らない規準の表と式だと、部の見出しで分かるようにする。
#[component]
pub(crate) fn ConcretePage() -> Element {
    let mut fc = use_signal(|| "24".to_string());
    let readout = ConcreteReadout::from_fc_input(&fc.read());
    let [unit_weight, young_modulus] = readout.values().map(str::to_string);
    let allowable_rows = readout.allowable_rows();
    let long_term_decimals = max_decimals(allowable_rows.iter().map(|row| row.long_term.as_str()));
    let short_term_decimals =
        max_decimals(allowable_rows.iter().map(|row| row.short_term.as_str()));
    let (error, warning) = match &readout {
        ConcreteReadout::Computed { warning, .. } => (None, warning.clone()),
        ConcreteReadout::Unusable { error } => (Some(error.clone()), None),
    };
    let fc_max = ConcreteKind::Normal
        .unit_weight_table()
        .last()
        .expect("表の区分は1つ以上")
        .fc_upper
        .as_n_per_mm2();
    rsx! {
        Page { route: Route::DataConcrete {},
            Columns {
                Column {
                    // 入力のすぐ下に結果（`space-6`）。この組と次の表の間は Column の間（`space-7`）
                    div { class: "flex flex-col gap-6",
                        div { class: "flex flex-col gap-1",
                            label { class: "flex flex-col gap-1",
                                span { class: if error.is_some() { "text-label text-ng" } else { "text-label text-muted" },
                                    "Fc（N/mm²）"
                                }
                                input {
                                    class: if error.is_some() {
                                        "self-start rounded-8 border-2 border-ng bg-paper px-3 py-2 font-mono text-num text-right"
                                    } else {
                                        "self-start rounded-8 border border-line-strong bg-paper px-3 py-2 font-mono text-num text-right"
                                    },
                                    r#type: "text",
                                    inputmode: "decimal",
                                    value: "{fc}",
                                    oninput: move |e| fc.set(e.value()),
                                }
                            }
                            if let Some(message) = &error {
                                p { class: "text-label text-ng", {message.clone()} }
                            }
                            if let Some(message) = warning {
                                p { class: "text-label", "! {message}" }
                            }
                        }
                        Table {
                            thead {
                                HeadRow {
                                    HeadCell { "気乾単位体積重量 γ（kN/m³）" }
                                    HeadCell { "ヤング係数 E（N/mm²）" }
                                }
                            }
                            tbody {
                                Row {
                                    NumCell { {unit_weight} }
                                    NumCell { {young_modulus} }
                                }
                            }
                        }
                    }
                    Block {
                        Caption { "許容応力度（N/mm²）" }
                        Table {
                            thead {
                                HeadRow {
                                    HeadCell { left: true, "" }
                                    HeadCell { "長期" }
                                    HeadCell { "短期" }
                                }
                            }
                            tbody {
                                for row in allowable_rows {
                                    Row {
                                        TextCell { {row.name} }
                                        DecimalCell { value: row.long_term, decimals: long_term_decimals }
                                        DecimalCell { value: row.short_term, decimals: short_term_decimals }
                                    }
                                }
                            }
                        }
                    }
                }
                Divider {}
                Column { id: "basis",
                    Block {
                        Caption { "気乾単位体積重量" }
                        Table {
                            thead {
                                HeadRow {
                                    HeadCell { left: true, "Fc（N/mm²）" }
                                    HeadCell { "γ（kN/m³）" }
                                }
                            }
                            tbody {
                                for (range, gamma) in unit_weight_rows() {
                                    Row {
                                        TextCell { {range} }
                                        NumCell { {gamma} }
                                    }
                                }
                            }
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
                            thead {
                                HeadRow {
                                    HeadCell { left: true, "" }
                                    HeadCell { left: true, "長期" }
                                    HeadCell { left: true, "短期" }
                                }
                            }
                            tbody {
                                for a in Allowable::ALL {
                                    Row {
                                        // 式が2行になる付着は、名前も括弧書きの前で改行する。1行では式と短期の列に押されて
                                        // スマホ幅で「（その他／の鉄筋）」と語の途中で切れたため
                                        TextCell {
                                            match a.name().split_once('（') {
                                                Some((head, rest)) => rsx! { {head} br {} "（{rest}" },
                                                None => rsx! { {a.name()} },
                                            }
                                        }
                                        FormulaCell { formula: a.long_term_formula() }
                                        FormulaCell { formula: Formula::Plain(a.short_term_formula()) }
                                    }
                                }
                            }
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
    let rows = designation_rows();
    let decimals = column_decimals(&rows);
    rsx! {
        Page { route: Route::DataRebar {},
            Columns {
                Column {
                    Block {
                        Caption { "異形棒鋼の寸法、単位質量及び節の許容限度" }
                        Table {
                            thead {
                                HeadRow {
                                    HeadCell { left: true, "呼び名" }
                                    HeadCell { "公称直径" br {} "d（mm）" }
                                    HeadCell { "公称周長" br {} "l（mm）" }
                                    HeadCell { "公称断面積" br {} "S（mm²）" }
                                    HeadCell { "単位質量" br {} "w（kg/m）" }
                                }
                            }
                            tbody {
                                for (name , values) in rows {
                                    Row {
                                        TextCell { {name} }
                                        for (value , decimals) in values.into_iter().zip(decimals) {
                                            DecimalCell { value, decimals }
                                        }
                                    }
                                }
                            }
                        }
                        Notes {
                            li { "l = 3.142d、S = 0.7854d²、w = 0.00785S" }
                            li { "JIS Z 8401 の規則A で丸める" }
                        }
                        // 出典: JIS G 3112:2025 表6（画面には出さない。ADR に書いてある）
                    }
                }
                Column {
                    Block {
                        Caption { "許容応力度（N/mm²）" }
                        div { class: "overflow-x-auto",
                            Table {
                                thead {
                                    GroupRow {
                                        HeadCell { left: true, rows: 2, "種類" }
                                        GroupCell { span: 3, "長期" }
                                        GroupCell { span: 2, "短期" }
                                    }
                                    HeadRow {
                                        HeadCell { span: 2, "引張・圧縮" }
                                        HeadCell { "せん断" br {} "補強" }
                                        HeadCell { "引張・" br {} "圧縮" }
                                        HeadCell { "せん断" br {} "補強" }
                                    }
                                }
                                tbody {
                                    for kind in RebarKind::ALL.iter().copied() {
                                        Row {
                                            TextCell { {kind.symbol()} }
                                            NumCell { {long_term_tension_compression(kind)} }
                                            ParenCell { {long_term_tension_compression_d29_and_up(kind)} }
                                            NumCell { {integer(kind.allowable_shear_reinforcement().long_term)} }
                                            NumCell { {integer(kind.allowable_tension_compression(BarDesignation::D25).short_term)} }
                                            NumCell { {integer(kind.allowable_shear_reinforcement().short_term)} }
                                        }
                                    }
                                }
                            }
                        }
                        Notes {
                            li { "* D29 以上の太さの鉄筋に対しては（ ）内の数値とする" }
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

/// 表 6.2 の値は整数で書かれているので、そのまま整数で出す
fn integer(stress: Stress) -> String {
    format!("{:.0}", stress.as_n_per_mm2())
}
