//! Data（`GLOSSARY.md`、`docs/adr/0005`）: 材料や規格の値を、表と、値を導く式で示すページ。
//! 数値はすべて `rustruct-types` から引き、ここでは表示の桁だけを決める（Issue #19）。

use dioxus::prelude::*;
use rustruct_types::concrete::{Concrete, ConcreteKind};
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
                warning: c.warning().map(|w| w.to_string()),
            },
            Err(error) => Self::Unusable { error },
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

/// 有効数字 `figures` 桁の文字列。末尾の 0 も桁として残す（0.560）。整数部が桁を超えるときは整数で（1140）。
/// 鉄筋の値は型の側で JIS の丸めを済ませているので、ここは桁をそろえて書くだけで値は変わらない
fn significant(x: f64, figures: i32) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let mut exponent = x.abs().log10().floor() as i32;
    let scale = 10_f64.powi(figures - 1 - exponent);
    let rounded = (x * scale).round() / scale;
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
        .unit_weight_range_uppers()
        .iter()
        .map(|&upper| {
            let c = Concrete::new(upper, ConcreteKind::Normal).expect("区分の上限は正の有限値");
            let upper = upper.as_n_per_mm2();
            let range = match lower {
                None => format!("Fc ≦ {upper}"),
                Some(lower) => format!("{lower} < Fc ≦ {upper}"),
            };
            lower = Some(upper);
            (
                range,
                format!("{:.1}", c.unit_weight().as_kn_per_m3()),
                format!("{:.1}", c.rc_unit_weight().as_kn_per_m3()),
            )
        })
        .collect()
}

/// `/data/concrete`。カルキュレータ、気乾単位体積重量の表、ヤング係数の式。
#[component]
pub(crate) fn ConcretePage() -> Element {
    let mut fc = use_signal(|| "24".to_string());
    let readout = ConcreteReadout::from_fc_input(&fc.read());
    let [unit_weight, rc_unit_weight, young_modulus] = readout.values().map(str::to_string);
    let (error, warning) = match &readout {
        ConcreteReadout::Computed { warning, .. } => (None, warning.clone()),
        ConcreteReadout::Unusable { error } => (Some(error.clone()), None),
    };
    let fc_max = ConcreteKind::Normal
        .unit_weight_range_uppers()
        .last()
        .expect("表の区分は1つ以上")
        .as_n_per_mm2();
    rsx! {
        Page { route: Route::DataConcrete {},
            section {
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
                dl { class: "mt-3 grid grid-cols-2 gap-x-4 gap-y-2 max-w-sm",
                    ResultRow { name: "γ", value: unit_weight, unit: "kN/m³" }
                    ResultRow { name: "γ₁", value: rc_unit_weight, unit: "kN/m³" }
                    ResultRow { name: "E", value: young_modulus, unit: "N/mm²" }
                }
            }
            section { class: "mt-7",
                h2 { class: "text-heading", "気乾単位体積重量 γ" }
                table { class: "mt-3",
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
                p { class: "mt-2 text-label text-muted",
                    "鉄筋コンクリートの単位体積重量 γ₁ = γ + 1。Fc {fc_max} を超えると最後の区分を使う。"
                }
            }
            section { class: "mt-7",
                h2 { class: "text-heading", "ヤング係数 E" }
                p { class: "mt-3 font-mono text-num", "E = 3.35×10⁴ × (γ/24)² × (Fc/60)^(1/3)" }
                p { class: "mt-2 text-label text-muted",
                    "RC 規準（2018 年版）。E と Fc は N/mm²、γ は kN/m³。"
                }
            }
        }
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
            section { class: "mt-7",
                h2 { class: "text-heading", "種類" }
                ul { class: "mt-3 flex flex-wrap gap-x-4 gap-y-2 font-mono text-num",
                    for kind in RebarKind::ALL.iter().copied() {
                        li { {kind.symbol()} }
                    }
                }
                p { class: "mt-2 text-label text-muted", "SD685R と SD785R は主にせん断補強筋に用いる。" }
            }
        }
    }
}
