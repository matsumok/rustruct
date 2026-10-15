//! Data（`GLOSSARY.md`、`docs/adr/0005`）: 材料や規格の値を、表と、値を導く式で示すページ。
//! 数値はすべて `rustruct-types` から引き、ここでは表示の桁だけを決める（Issue #19）。

use dioxus::prelude::*;
use rustruct_types::allowable_stress::AllowableStress;
use rustruct_types::concrete::{BarPosition, Concrete, ConcreteKind};
use rustruct_types::quantity::Stress;
use rustruct_types::rebar::{BarDesignation, RebarKind};

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
    fn long_term_formula(self) -> &'static str {
        match self {
            Self::Compression => "Fc/3",
            Self::Shear => "min(Fc/30, 0.49+Fc/100)",
            Self::BondTop => "min(Fc/15, 0.9+2Fc/75)",
            Self::BondOther => "min(Fc/10, 1.35+Fc/25)",
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

/// 有効数字 `figures` 桁の文字列。末尾の 0 も桁として残す（0.560）。整数部が桁を超えるときは整数で（1140）。
/// 鉄筋の値は型の側で JIS の丸めを済ませているので、ここは桁をそろえて書くだけで値は変わらない
fn significant(x: f64, figures: i32) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let mut exponent = x.abs().log10().floor() as i32;
    let scale = 10_f64.powi(figures - 1 - exponent);
    // 2 進の誤差を落としてから四捨五入する（3.465 が 3.4649999… になって 3.46 に落ちないように）
    let rounded = ((x * scale * 1e6).round() / 1e6).round() / scale;
    if rounded.abs() >= 10_f64.powi(exponent + 1) {
        // 9.996 → 10.0 のように丸めで桁が上がった
        exponent += 1;
    }
    let decimals = (figures - 1 - exponent).max(0) as usize;
    format!("{rounded:.decimals$}")
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
                Column { first: true,
                    label { class: "block",
                        span { class: if error.is_some() { "block text-label text-ng" } else { "block text-label text-muted" },
                            "Fc（N/mm²）"
                        }
                        input {
                            class: if error.is_some() {
                                "mt-1 w-32 rounded-8 border-2 border-ng bg-paper px-3 py-2 font-mono text-num text-right"
                            } else {
                                "mt-1 w-32 rounded-8 border border-line-strong bg-paper px-3 py-2 font-mono text-num text-right"
                            },
                            r#type: "text",
                            inputmode: "decimal",
                            value: "{fc}",
                            oninput: move |e| fc.set(e.value()),
                        }
                    }
                    if let Some(message) = &error {
                        p { class: "mt-1 text-label text-ng", {message.clone()} }
                    }
                    if let Some(message) = warning {
                        p { class: "mt-1 text-label", "! {message}" }
                    }
                    div { class: "mt-6",
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
                    Caption { "許容応力度（N/mm²）" }
                    Table {
                        thead {
                            HeadRow {
                                HeadCell { first: true, "" }
                                HeadCell { "長期" }
                                HeadCell { "短期" }
                            }
                        }
                        tbody {
                            for row in allowable_rows {
                                Row {
                                    TextCell { first: true, {row.name} }
                                    NumCell { {row.long_term} }
                                    NumCell { {row.short_term} }
                                }
                            }
                        }
                    }
                }
                Column { id: "basis", divided: true,
                    Caption { first: true, "気乾単位体積重量" }
                    Table {
                        thead {
                            HeadRow {
                                HeadCell { first: true, "Fc（N/mm²）" }
                                HeadCell { "γ（kN/m³）" }
                            }
                        }
                        tbody {
                            for (range, gamma) in unit_weight_rows() {
                                Row {
                                    TextCell { first: true, {range} }
                                    NumCell { {gamma} }
                                }
                            }
                        }
                    }
                    Notes {
                        li { "Fc {fc_max} を超えると最後の区分を使う" }
                    }
                    // 出典: 実務の慣例の値（画面には出さない。ADR に書いてある）

                    Caption { "ヤング係数（N/mm²）" }
                    p { class: "font-mono text-num",
                        "E = 3.35×10⁴ × (γ/24)² × (Fc/60)"
                        sup { "1/3" }
                    }
                    // 出典: RC 規準（2024 年版）（画面には出さない。ADR に書いてある）

                    Caption { "許容応力度（N/mm²）" }
                    Table {
                        thead {
                            HeadRow {
                                HeadCell { first: true, "" }
                                HeadCell { formula: true, "長期" }
                                HeadCell { formula: true, "短期" }
                            }
                        }
                        tbody {
                            for a in Allowable::ALL {
                                Row {
                                    TextCell { first: true, {a.name()} }
                                    FormulaCell { {a.long_term_formula()} }
                                    FormulaCell { {a.short_term_formula()} }
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

/// 本文の列を並べる（ui-decisions「列の単位」）。PC では1単位（`width-column`）の列を `space-4` の溝で横に、
/// スマホでは縦に積む。
#[component]
fn Columns(children: Element) -> Element {
    rsx! {
        div { class: "md:flex md:items-start md:gap-x-4", {children} }
    }
}

/// 本文の1列。スマホでは画面いっぱい、iPad からは1単位の幅（`docs/adr/0007`）。スマホで縦に積むとき、2列目からは `space-7` を空ける。
/// `divided` の列は前の列との間に `line` の区切りを引く（スマホでは上、PC では左）。スマホの区切りの上下は
/// `space-8`。`space-7` は表の行の高さとほぼ同じで、区切りが表の次の行の罫に見えたため。入力と結果の列と、
/// 入力に依らない根拠の列を分けるのに使う。区切りの余白と線は1単位の外に足し（`box-content`）、中身は1単位のまま。`id` を付けると住所の `#id` でその列へ飛べる。
#[component]
fn Column(
    #[props(default)] first: bool,
    #[props(default)] divided: bool,
    #[props(default)] id: &'static str,
    children: Element,
) -> Element {
    let class = match (first, divided) {
        (true, _) => "w-full min-w-0 md:max-w-column",
        (false, false) => "mt-7 md:mt-0 w-full min-w-0 md:max-w-column",
        (false, true) => {
            "mt-8 pt-8 border-t border-line md:mt-0 md:pt-0 md:border-t-0 md:border-l md:pl-4 md:box-content w-full min-w-0 md:max-w-column"
        }
    };
    rsx! {
        div { class, id: (!id.is_empty()).then_some(id), {children} }
    }
}

/// 表や式の見出し。`heading`（太字・ink）。中身との間は `space-2`、前の中身との間は `space-7`。
/// 部の見出し（入力・結果・根拠）は置かず、この見出しで区切る（ui-decisions「列の単位と表」）。
#[component]
fn Caption(#[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        h2 { class: if first { "mb-2 text-heading" } else { "mt-7 mb-2 text-heading" },
            {children}
        }
    }
}

/// 表や式に添える注記の箇条。
#[component]
fn Notes(children: Element) -> Element {
    rsx! {
        ul { class: "mt-2 list-disc pl-4 text-label text-muted", {children} }
    }
}

/// 表示用の表（ui-decisions「列の単位と表」）。横罫だけで、線はすべて `line`（`ink` は部の見出しの上だけ）。
/// 縦罫と塗りは使わない。幅は列いっぱい（1単位）。入りきらない表は外側を `overflow-x-auto` で包む。
#[component]
fn Table(children: Element) -> Element {
    rsx! {
        table { class: "w-full", {children} }
    }
}

/// 表の見出しの行。下に `line`。
#[component]
fn HeadRow(children: Element) -> Element {
    rsx! {
        tr { class: "border-b border-line", {children} }
    }
}

/// 表の本体の行。下に `line`（最後の行も）。
#[component]
fn Row(children: Element) -> Element {
    rsx! {
        tr { class: "border-b border-line", {children} }
    }
}

/// 列見出し。`label` の muted。`first` の列は左、`formula` の列は式（`FormulaCell`）に合わせて左、
/// ほかは数値の列なので右に揃える。セルの左右は `space-3`、表の端の列は端に寄せる。
#[component]
fn HeadCell(
    #[props(default)] first: bool,
    #[props(default)] formula: bool,
    children: Element,
) -> Element {
    let class = match (first, formula) {
        (true, _) => "py-2 pr-3 text-left text-label text-muted font-normal",
        (false, true) => "py-2 pl-3 text-left text-label text-muted font-normal",
        (false, false) => "py-2 pl-3 last:pr-0 text-right text-label text-muted font-normal",
    };
    rsx! {
        th {
            class,
            {children}
        }
    }
}

/// 名前や区分のセル。`body` で左揃え。
#[component]
fn TextCell(#[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        td { class: if first { "py-2 pr-3 text-body" } else { "py-2 px-3 text-body" }, {children} }
    }
}

/// 数値のセル。`num` の等幅で右揃え。
#[component]
fn NumCell(#[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        td {
            class: if first { "py-2 pr-3 font-mono text-num whitespace-nowrap" } else { "py-2 pl-3 font-mono text-num text-right whitespace-nowrap" },
            {children}
        }
    }
}

/// 式のセル。`num` の等幅で左揃え、折り返さない。
#[component]
fn FormulaCell(children: Element) -> Element {
    rsx! {
        td { class: "py-2 pl-3 font-mono text-num whitespace-nowrap", {children} }
    }
}

/// `/data/rebar`。呼び名の表と、種類ごとの許容応力度の表。値は JIS と RC 規準の表と同じ桁で出す。
#[component]
pub(crate) fn RebarPage() -> Element {
    rsx! {
        Page { route: Route::DataRebar {},
            Columns {
                Column { first: true,
                    Caption { first: true, "呼び名" }
                        div {
                        Table {
                            thead {
                                HeadRow {
                                    HeadCell { first: true, "呼び名" }
                                    HeadCell { "公称直径" br {} "d（mm）" }
                                    HeadCell { "公称周長" br {} "l（mm）" }
                                    HeadCell { "公称断面積" br {} "S（mm²）" }
                                    HeadCell { "単位質量" br {} "w（kg/m）" }
                                }
                            }
                            tbody {
                                for designation in BarDesignation::ALL.iter().copied() {
                                    Row {
                                        TextCell { first: true, {designation.name()} }
                                        NumCell { {significant(designation.nominal_diameter().as_mm(), 3)} }
                                        NumCell { {format!("{:.1}", designation.nominal_perimeter().as_mm())} }
                                        NumCell { {significant(designation.nominal_area().as_mm2(), 4)} }
                                        NumCell { {significant(designation.unit_mass().as_kg_per_m(), 3)} }
                                    }
                                }
                            }
                        }
                        Notes {
                            li {
                                "l = 3.142d、S = 0.7854d²、w = 0.00785S"
                                br {}
                                "JIS Z 8401 の規則A で丸める"
                            }
                        }
                        // 出典: JIS G 3112:2025 表6（画面には出さない。ADR に書いてある）
                    }
                }
                Column {
                    Caption { first: true, "許容応力度（N/mm²）" }
                        div {
                        div { class: "overflow-x-auto",
                            Table {
                                thead {
                                    HeadRow {
                                        HeadCell { first: true, "種類" }
                                        HeadCell { "長期 引張・圧縮" }
                                        HeadCell { "長期 せん断補強" }
                                        HeadCell { "短期 引張・圧縮" }
                                        HeadCell { "短期 せん断補強" }
                                    }
                                }
                                tbody {
                                    for kind in RebarKind::ALL.iter().copied() {
                                        Row {
                                            TextCell { first: true, {kind.symbol()} }
                                            NumCell { {long_term_tension_compression(kind)} }
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

/// 長期の引張・圧縮。D29 以上で値が変わる種類は、表 6.2 と同じく「215（*195）」と書き、凡例を注記に置く
fn long_term_tension_compression(kind: RebarKind) -> String {
    let below = integer(
        kind.allowable_tension_compression(BarDesignation::D25)
            .long_term,
    );
    let d29_and_up = integer(
        kind.allowable_tension_compression(BarDesignation::D29)
            .long_term,
    );
    if below == d29_and_up {
        below
    } else {
        format!("{below}（*{d29_and_up}）")
    }
}

/// 表 6.2 の値は整数で書かれているので、そのまま整数で出す
fn integer(stress: Stress) -> String {
    format!("{:.0}", stress.as_n_per_mm2())
}
