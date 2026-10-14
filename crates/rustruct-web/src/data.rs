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
        /// 許容応力度。圧縮、せん断、付着（上端筋）、付着（その他の鉄筋）の順
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
                allowable_stresses: {
                    let stresses = [
                        c.allowable_compression(),
                        c.allowable_shear(),
                        c.allowable_bond(BarPosition::Top),
                        c.allowable_bond(BarPosition::Other),
                    ];
                    ALLOWABLE_NAMES
                        .into_iter()
                        .zip(stresses)
                        .map(|(name, stress)| AllowableRow::new(name, stress))
                        .collect()
                },
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
            Self::Unusable { .. } => ALLOWABLE_NAMES
                .map(|name| AllowableRow {
                    name,
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

/// 許容応力度の表の行の名前。`ConcreteReadout::Computed` の `allowable_stresses` の順
const ALLOWABLE_NAMES: [&str; 4] = ["圧縮", "せん断", "付着（上端筋）", "付着（その他の鉄筋）"];

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
                dl { class: "grid grid-cols-2 gap-x-4 gap-y-2 max-w-sm",
                    ResultRow { name: "γ", value: unit_weight, unit: "kN/m³" }
                    ResultRow { name: "γ₁", value: rc_unit_weight, unit: "kN/m³" }
                    ResultRow { name: "E", value: young_modulus, unit: "N/mm²" }
                }
                Caption { "許容応力度（N/mm²）" }
                table { class: "mt-2",
                    thead {
                        tr { class: "border-b border-line",
                            HeadCell { first: true, "" }
                            HeadCell { "長期" }
                            HeadCell { "短期" }
                        }
                    }
                    tbody {
                        for row in allowable_rows {
                            tr { class: "border-b border-line",
                                td { class: "py-2 pr-3 text-body", {row.name} }
                                NumCell { {row.long_term} }
                                NumCell { {row.short_term} }
                            }
                        }
                    }
                }
            }
            Part { title: "根拠",
                Caption { first: true, "気乾単位体積重量 γ と鉄筋コンクリートの単位体積重量 γ₁" }
                table { class: "mt-2",
                    thead {
                        tr { class: "border-b border-line",
                            HeadCell { first: true, "Fc（N/mm²）" }
                            HeadCell { "γ（kN/m³）" }
                            HeadCell { "γ₁（kN/m³）" }
                        }
                    }
                    tbody {
                        for (range, gamma, gamma1) in unit_weight_rows() {
                            tr { class: "border-b border-line",
                                NumCell { first: true, {range} }
                                NumCell { {gamma} }
                                NumCell { {gamma1} }
                            }
                        }
                    }
                }
                Note { "γ は慣例の値。γ₁ = γ + 1。Fc {fc_max} を超えると最後の区分を使う。" }
                Caption { "ヤング係数 E" }
                p { class: "mt-2 font-mono text-num", "E = 3.35×10⁴ × (γ/24)² × (Fc/60)^(1/3)" }
                Note { "RC 規準（2024 年版）。E と Fc は N/mm²、γ は kN/m³。" }
                Caption { "許容応力度" }
                table { class: "mt-2",
                    thead {
                        tr { class: "border-b border-line",
                            HeadCell { first: true, "" }
                            HeadCell { first: true, "長期" }
                            HeadCell { first: true, "短期" }
                        }
                    }
                    tbody {
                        for (name, long, short) in ALLOWABLE_FORMULAS {
                            tr { class: "border-b border-line align-top",
                                td { class: "py-2 pr-3 text-body", {name} }
                                td { class: "py-2 pr-3 text-body", {long} }
                                td { class: "py-2 text-body", {short} }
                            }
                        }
                    }
                }
                Note {
                    "RC 規準（2024 年版）表 6.1・6.3。付着は異形鉄筋。上端筋は曲げ材でその鉄筋の下に 300 mm 以上のコンクリートが打ち込まれる水平鉄筋。かぶりが鉄筋径の 1.5 倍未満なら付着に「かぶり厚さ/(鉄筋径の 1.5 倍)」を掛ける。"
                }
            }
        }
    }
}

/// 許容応力度の根拠の式: (名前, 長期, 短期)。名前は `ALLOWABLE_NAMES` と同じ順
const ALLOWABLE_FORMULAS: [(&str, &str, &str); 4] = [
    (ALLOWABLE_NAMES[0], "Fc/3", "長期の 2 倍"),
    (
        ALLOWABLE_NAMES[1],
        "Fc/30 かつ 0.49 + Fc/100 以下",
        "長期の 1.5 倍",
    ),
    (
        ALLOWABLE_NAMES[2],
        "Fc/15 かつ 0.9 + 2Fc/75 以下",
        "長期の 1.5 倍",
    ),
    (
        ALLOWABLE_NAMES[3],
        "Fc/10 かつ 1.35 + Fc/25 以下",
        "長期の 1.5 倍",
    ),
];

/// ページの部（入力・結果・根拠）。見出しの下に区切り線を引き、部と部の間は `space-7`。
#[component]
fn Part(title: &'static str, #[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        section { class: if first { "" } else { "mt-7" },
            h2 { class: "text-heading pb-2 border-b border-line", {title} }
            div { class: "mt-3", {children} }
        }
    }
}

/// 部の中の小見出し（表や式の名前）。前の中身との間は `space-6`。
#[component]
fn Caption(#[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        h3 { class: if first { "text-label text-muted" } else { "mt-6 text-label text-muted" },
            {children}
        }
    }
}

/// 表や式に添える注記。
#[component]
fn Note(children: Element) -> Element {
    rsx! {
        p { class: "mt-2 text-label text-muted", {children} }
    }
}

/// 結果の1行。名前と、`num` の値に `line-strong` の下線（`ResultValue`）と単位。
#[component]
fn ResultRow(name: &'static str, value: String, unit: &'static str) -> Element {
    rsx! {
        dt { class: "text-body", {name} }
        dd { class: "font-mono text-num text-right border-b border-line-strong",
            {value}
            span { class: "text-muted", " {unit}" }
        }
    }
}

/// 表示用の表の列見出し。先頭の列は左、ほかは数値の列なので右に揃える。
#[component]
fn HeadCell(#[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        th {
            class: if first { "py-2 pr-3 text-left text-label text-muted font-normal" } else { "py-2 pl-3 text-right text-label text-muted font-normal" },
            {children}
        }
    }
}

/// 表示用の表のセル。`num` の等幅で、先頭の列は左、ほかは右に揃える。
#[component]
fn NumCell(#[props(default)] first: bool, children: Element) -> Element {
    rsx! {
        td {
            class: if first { "py-2 pr-3 font-mono text-num" } else { "py-2 pl-3 font-mono text-num text-right" },
            {children}
        }
    }
}

/// `/data/rebar`。呼び名の表と種類の一覧。値は JIS の表と同じ桁で出す。
#[component]
pub(crate) fn RebarPage() -> Element {
    rsx! {
        Page { route: Route::DataRebar {},
            section { class: "overflow-x-auto",
                h2 { class: "text-heading", "呼び名" }
                table { class: "mt-3",
                    thead {
                        tr { class: "border-b border-line",
                            HeadCell { first: true, "呼び名" }
                            HeadCell { "d（mm）" }
                            HeadCell { "l（mm）" }
                            HeadCell { "S（mm²）" }
                            HeadCell { "w（kg/m）" }
                        }
                    }
                    tbody {
                        for designation in BarDesignation::ALL.iter().copied() {
                            tr { class: "border-b border-line",
                                NumCell { first: true, {designation.name()} }
                                NumCell { {significant(designation.nominal_diameter().as_mm(), 3)} }
                                NumCell { {format!("{:.1}", designation.nominal_perimeter().as_mm())} }
                                NumCell { {significant(designation.nominal_area().as_mm2(), 4)} }
                                NumCell { {significant(designation.unit_mass().as_kg_per_m(), 3)} }
                            }
                        }
                    }
                }
                p { class: "mt-2 text-label text-muted",
                    "JIS G 3112:2025 表6。l = 3.142d、S = 0.7854d²、w = 0.00785S を JIS Z 8401 の規則A で丸める。"
                }
            }
            section { class: "mt-7 overflow-x-auto",
                h2 { class: "text-heading", "許容応力度（N/mm²）" }
                table { class: "mt-3",
                    thead {
                        tr { class: "border-b border-line",
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
                            tr { class: "border-b border-line",
                                NumCell { first: true, {kind.symbol()} }
                                NumCell { {integer(kind.allowable_tension_compression(BarDesignation::D25).long_term)} }
                                NumCell { {integer(kind.allowable_tension_compression(BarDesignation::D29).long_term)} }
                                NumCell { {integer(kind.allowable_shear_reinforcement().long_term)} }
                                NumCell { {integer(kind.allowable_tension_compression(BarDesignation::D25).short_term)} }
                                NumCell { {integer(kind.allowable_shear_reinforcement().short_term)} }
                            }
                        }
                    }
                }
                p { class: "mt-2 text-label text-muted",
                    "RC 規準 表 6.2。長期の引張・圧縮は D29 以上の太さで小さくなる。SD590A 以上は表に値が無いので載せない。"
                }
            }
        }
    }
}

/// 表 6.2 の値は整数で書かれているので、そのまま整数で出す
fn integer(stress: Stress) -> String {
    format!("{:.0}", stress.as_n_per_mm2())
}
