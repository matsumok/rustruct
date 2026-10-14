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
        /// 鉄筋コンクリートの単位体積重量 γ₁ [kN/m³]
        rc_unit_weight: String,
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
                rc_unit_weight: format!("{:.1}", c.rc_unit_weight().as_kn_per_m3()),
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

    /// (γ, γ₁, E) の表示。使えないときはすべて「—」
    fn values(&self) -> [&str; 3] {
        match self {
            Self::Computed {
                unit_weight,
                rc_unit_weight,
                young_modulus,
                ..
            } => [unit_weight, rc_unit_weight, young_modulus],
            Self::Unusable { .. } => [NOT_COMPUTED; 3],
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

/// 気乾単位体積重量の表の行: (Fc の区分, γ, γ₁)。区分の上限で γ を引く
fn unit_weight_rows() -> Vec<(String, String, String)> {
    let mut lower: Option<f64> = None;
    ConcreteKind::Normal
        .unit_weight_table()
        .iter()
        .map(|row| {
            // γ₁ は区分の上限の Fc のコンクリートに求めさせる（γ + 1 をここで書かない）
            let c =
                Concrete::new(row.fc_upper, ConcreteKind::Normal).expect("区分の上限は正の有限値");
            let upper = row.fc_upper.as_n_per_mm2();
            let range = match lower {
                None => format!("Fc ≦ {upper}"),
                Some(lower) => format!("{lower} < Fc ≦ {upper}"),
            };
            lower = Some(upper);
            (
                range,
                format!("{:.1}", row.unit_weight.as_kn_per_m3()),
                format!("{:.1}", c.rc_unit_weight().as_kn_per_m3()),
            )
        })
        .collect()
}

/// `/data/concrete`。入力（Fc）、結果（γ・γ₁・E・許容応力度）、根拠（表と式）の順に分けて置く。
/// 結果は入力から出た値、根拠は入力に依らない規準の表と式だと、部の見出しで分かるようにする。
#[component]
pub(crate) fn ConcretePage() -> Element {
    let mut fc = use_signal(|| "24".to_string());
    let readout = ConcreteReadout::from_fc_input(&fc.read());
    let [unit_weight, rc_unit_weight, young_modulus] = readout.values().map(str::to_string);
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
                    Part { title: "入力", first: true,
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
                    }
                    Part { title: "結果",
                        Table {
                            tbody {
                                Row {
                                    TextCell { first: true, "気乾単位体積重量 γ（kN/m³）" }
                                    NumCell { {unit_weight} }
                                }
                                Row {
                                    TextCell { first: true, "鉄筋コンクリートの単位体積重量 γ₁（kN/m³）" }
                                    NumCell { {rc_unit_weight} }
                                }
                                Row {
                                    TextCell { first: true, "ヤング係数 E（N/mm²）" }
                                    NumCell { {young_modulus} }
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
                }
                Column {
                    Part { title: "根拠", id: "basis", first: true,
                        Caption { first: true, "単位体積重量" }
                        Table {
                            thead {
                                HeadRow {
                                    HeadCell { first: true, "Fc（N/mm²）" }
                                    HeadCell { "γ（kN/m³）" }
                                    HeadCell { "γ₁（kN/m³）" }
                                }
                            }
                            tbody {
                                for (range, gamma, gamma1) in unit_weight_rows() {
                                    Row {
                                        TextCell { first: true, {range} }
                                        NumCell { {gamma} }
                                        NumCell { {gamma1} }
                                    }
                                }
                            }
                        }
                        Notes {
                            li { "γ は気乾単位体積重量、γ₁ = γ + 1 は鉄筋コンクリートの単位体積重量" }
                            li { "Fc {fc_max} を超えると最後の区分を使う" }
                        }
                        Source { "実務の慣例の値" }

                        Caption { "ヤング係数（N/mm²）" }
                        p { class: "font-mono text-num",
                            "E = 3.35×10⁴ × (γ/24)² × (Fc/60)"
                            sup { "1/3" }
                        }
                        Notes {
                            li { "Fc は N/mm²、γ は kN/m³" }
                        }
                        Source { "RC 規準（2024 年版）" }

                        Caption { "許容応力度（長期、N/mm²）" }
                        Table {
                            tbody {
                                for a in Allowable::ALL {
                                    Row {
                                        TextCell { first: true, {a.name()} }
                                        FormulaCell { {a.long_term_formula()} }
                                    }
                                }
                            }
                        }
                        Notes {
                            li { "短期は長期の 1.5 倍。圧縮だけ 2 倍" }
                            li { "付着は異形鉄筋。上端筋は、曲げ材でその鉄筋の下に 300 mm 以上のコンクリートが打ち込まれる水平鉄筋" }
                            li { "かぶりが鉄筋径の 1.5 倍未満なら、付着に かぶり厚さ /（鉄筋径 × 1.5）を掛ける" }
                        }
                        Source { "RC 規準（2024 年版）表 6.1・6.3" }
                    }
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

/// 本文の1列。幅は1単位まで。スマホで縦に積むとき、2列目からは部と部の間（`space-7`）を空ける。
#[component]
fn Column(#[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        div { class: if first { "w-full min-w-0 max-w-column" } else { "mt-7 md:mt-0 w-full min-w-0 max-w-column" },
            {children}
        }
    }
}

/// ページの部（入力・結果・根拠）。デザインシステムの SectionHead: 見出しの上に `ink` の1本線、
/// 中身との間は `space-3`、前の部との間は `space-7`。`id` を付けると住所の `#id` でその部へ飛べる。
#[component]
fn Part(
    title: &'static str,
    #[props(default)] first: bool,
    #[props(default)] id: &'static str,
    children: Element,
) -> Element {
    rsx! {
        section { class: if first { "" } else { "mt-7" }, id: (!id.is_empty()).then_some(id),
            h2 { class: "border-t border-ink pt-2 text-heading", {title} }
            div { class: "mt-3", {children} }
        }
    }
}

/// 表や式の名前。上に置き、中身との間は `space-2`。前の中身との間は `space-6`。
#[component]
fn Caption(#[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        h3 { class: if first { "mb-2 text-label text-muted" } else { "mt-6 mb-2 text-label text-muted" },
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

/// 表や式の出典。注記の後に1行で。
#[component]
fn Source(children: Element) -> Element {
    rsx! {
        p { class: "mt-1 text-label text-muted", "出典: " {children} }
    }
}

/// 表示用の表（ui-decisions「表」）。横罫だけで、表の頭と尻に `ink`、見出しの下と行の間に `line`。
/// 縦罫と塗りは使わない。幅は列いっぱい（1単位）。入りきらない表は外側を `overflow-x-auto` で包む。
#[component]
fn Table(children: Element) -> Element {
    rsx! {
        table { class: "w-full border-y border-ink", {children} }
    }
}

/// 表の見出しの行。下に `line`。
#[component]
fn HeadRow(children: Element) -> Element {
    rsx! {
        tr { class: "border-b border-line", {children} }
    }
}

/// 表の本体の行。行の間に `line`。最後の行は表の尻の `ink` に任せる。
#[component]
fn Row(children: Element) -> Element {
    rsx! {
        tr { class: "border-b border-line last:border-b-0", {children} }
    }
}

/// 列見出し。`label` の muted。`first` の列は左、ほかは数値の列なので右に揃える。
/// セルの左右は `space-3`、表の端の列は端に寄せる。
#[component]
fn HeadCell(#[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        th {
            class: if first { "py-2 pr-3 text-left text-label text-muted font-normal" } else { "py-2 pl-3 last:pr-0 text-right text-label text-muted font-normal" },
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
                    Part { title: "呼び名", first: true,
                        Table {
                            thead {
                                HeadRow {
                                    HeadCell { first: true, "呼び名" }
                                    HeadCell { "d（mm）" }
                                    HeadCell { "l（mm）" }
                                    HeadCell { "S（mm²）" }
                                    HeadCell { "w（kg/m）" }
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
                            li { "l = 3.142d、S = 0.7854d²、w = 0.00785S を JIS Z 8401 の規則A で丸める" }
                        }
                        Source { "JIS G 3112:2025 表6" }
                    }
                }
                Column {
                    Part { title: "許容応力度（N/mm²）", first: true,
                        div { class: "overflow-x-auto",
                            Table {
                                thead {
                                    HeadRow {
                                        HeadCell { first: true, "種類" }
                                        HeadCell { "長期 引張・圧縮" }
                                        HeadCell { "長期 D29 以上" }
                                        HeadCell { "長期 せん断補強" }
                                        HeadCell { "短期 引張・圧縮" }
                                        HeadCell { "短期 せん断補強" }
                                    }
                                }
                                tbody {
                                    for kind in RebarKind::ALL.iter().copied() {
                                        Row {
                                            TextCell { first: true, {kind.symbol()} }
                                            NumCell { {integer(kind.allowable_tension_compression(BarDesignation::D25).long_term)} }
                                            NumCell { {integer(kind.allowable_tension_compression(BarDesignation::D29).long_term)} }
                                            NumCell { {integer(kind.allowable_shear_reinforcement().long_term)} }
                                            NumCell { {integer(kind.allowable_tension_compression(BarDesignation::D25).short_term)} }
                                            NumCell { {integer(kind.allowable_shear_reinforcement().short_term)} }
                                        }
                                    }
                                }
                            }
                        }
                        Notes {
                            li { "長期の引張・圧縮は D29 以上の太さで小さくなる" }
                            li { "SD590A 以上は表に値が無いので載せない" }
                        }
                        Source { "RC 規準（2024 年版）表 6.2" }
                    }
                }
            }
        }
    }
}

/// 表 6.2 の値は整数で書かれているので、そのまま整数で出す
fn integer(stress: Stress) -> String {
    format!("{:.0}", stress.as_n_per_mm2())
}
