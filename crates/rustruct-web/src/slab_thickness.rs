//! Tool「必要スラブ厚さ」（`/tools/slab-thickness`）。言葉は `docs/tools/slab-thickness/GLOSSARY.md`。
//! 数値はすべて `rustruct-calc` の `shared_values`・`evaluate` から引き、ここでは入力の文字の読み取りと、
//! 表示の桁だけを決める。計算はブラウザの中で同期に回し、入力を変えるたびに結果を描き直す。
//!
//! 今は検討点を 1 つだけ入れる。検討点のシートと等厚曲線の図は後の Issue。

use dioxus::prelude::*;
use rustruct_calc::slab_thickness::{
    Conditions, CriteriaSet, Criterion, Evaluation, SharedValues, Spans, evaluate, shared_values,
};
use rustruct_types::concrete::{Concrete, ConcreteKind};
use rustruct_types::quantity::{AreaLoad, Length, Stress};

use crate::sheet::{Sheet, SheetCell, SheetHead, SheetHeadRow, SheetInput, SheetRow};
use crate::table::{
    Block, Caption, Column, Columns, HeadCell, HeadRow, Notes, NumCell, Row, Table, TextCell,
    thousands,
};
use crate::{Page, Route};

/// 計算できない値の表示（`docs/ui-decisions.md`「エラーと警告」）
const NOT_COMPUTED: &str = "—";

/// 数値の入力欄。並びは条件の Sheet の行の順、検討点の Sheet の列の順
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabField {
    Fc,
    FinishLoad,
    LiveLoad,
    Amplification,
    /// 変形角の分母 n（1/n）
    DeflectionRatio,
    DeflectionAbsolute,
    SpanX,
    SpanY,
}

impl SlabField {
    /// 条件の Sheet の数値の欄。この後に表18-1 のチェックが続く
    const CONDITIONS: [Self; 6] = [
        Self::Fc,
        Self::FinishLoad,
        Self::LiveLoad,
        Self::Amplification,
        Self::DeflectionRatio,
        Self::DeflectionAbsolute,
    ];

    /// 欄の名前。単位は括弧で名前に付ける
    pub fn label(self) -> &'static str {
        match self {
            Self::Fc => "Fc（N/mm²）",
            Self::FinishLoad => "仕上げ荷重（N/m²）",
            Self::LiveLoad => "積載荷重（N/m²）",
            Self::Amplification => "変形増大係数",
            Self::DeflectionRatio => "変形角 1/",
            Self::DeflectionAbsolute => "たわみ絶対量（mm）",
            Self::SpanX => "Lx（m）",
            Self::SpanY => "Ly（m）",
        }
    }

    /// 空欄を「判定に加えない」と読む欄（クライテリアのうち値を持つもの）
    fn may_be_blank(self) -> bool {
        matches!(self, Self::DeflectionRatio | Self::DeflectionAbsolute)
    }

    /// 0 を受ける欄（荷重）。ほかは 0 より大きい値だけ
    fn accepts_zero(self) -> bool {
        matches!(self, Self::FinishLoad | Self::LiveLoad)
    }
}

/// 表18-1 のチェックの名前
const MIN_THICKNESS_LABEL: &str = "表18-1 最小厚さ";

/// 検討点の名称の欄の名前
const NAME_LABEL: &str = "名称";

/// クライテリアが 1 つも無いときの文言
const NO_CRITERIA: &str = "クライテリアを 1 つ以上入れる";

/// 入力の中身。数値の欄は打ったままの文字で持ち、読めるかは `SlabReadout` が決める
#[derive(Debug, Clone, PartialEq)]
pub struct SlabInputs {
    pub fc: String,
    pub finish_load: String,
    pub live_load: String,
    pub amplification: String,
    /// 変形角の分母 n。空欄なら変形角を判定に加えない
    pub deflection_ratio: String,
    /// 空欄ならたわみ絶対量を判定に加えない
    pub deflection_absolute: String,
    pub min_thickness: bool,
    pub name: String,
    pub span_x: String,
    pub span_y: String,
}

impl Default for SlabInputs {
    /// 開いたときの値。移植元の slab のとおり（Issue #17）
    fn default() -> Self {
        Self {
            fc: "24".into(),
            finish_load: "1000".into(),
            live_load: "1800".into(),
            amplification: "16".into(),
            deflection_ratio: "400".into(),
            deflection_absolute: String::new(),
            min_thickness: false,
            name: "S1".into(),
            span_x: "4.0".into(),
            span_y: "5.0".into(),
        }
    }
}

impl SlabInputs {
    fn get(&self, field: SlabField) -> &str {
        match field {
            SlabField::Fc => &self.fc,
            SlabField::FinishLoad => &self.finish_load,
            SlabField::LiveLoad => &self.live_load,
            SlabField::Amplification => &self.amplification,
            SlabField::DeflectionRatio => &self.deflection_ratio,
            SlabField::DeflectionAbsolute => &self.deflection_absolute,
            SlabField::SpanX => &self.span_x,
            SlabField::SpanY => &self.span_y,
        }
    }

    fn set(&mut self, field: SlabField, value: String) {
        *match field {
            SlabField::Fc => &mut self.fc,
            SlabField::FinishLoad => &mut self.finish_load,
            SlabField::LiveLoad => &mut self.live_load,
            SlabField::Amplification => &mut self.amplification,
            SlabField::DeflectionRatio => &mut self.deflection_ratio,
            SlabField::DeflectionAbsolute => &mut self.deflection_absolute,
            SlabField::SpanX => &mut self.span_x,
            SlabField::SpanY => &mut self.span_y,
        } = value;
    }
}

/// 1 つの欄を数に読む。3 桁区切りの「,」は読み飛ばす。空欄を受ける欄の空欄は `None`。
/// 読めない理由は欄に添える文言
fn read_value(field: SlabField, input: &str) -> Result<Option<f64>, &'static str> {
    let input = input.trim().replace(',', "");
    if input.is_empty() {
        return if field.may_be_blank() {
            Ok(None)
        } else {
            Err("値を入れる")
        };
    }
    let value: f64 = input
        .parse()
        .ok()
        .filter(|v: &f64| v.is_finite())
        .ok_or("数値で入れる")?;
    if field.accepts_zero() {
        if value < 0.0 {
            return Err("0 以上の値を入れる");
        }
    } else if value <= 0.0 {
        return Err("0 より大きい値を入れる");
    }
    Ok(Some(value))
}

/// 入力から求めた表示。数値は丸めて文字にしたもの。計算できないときは「—」
#[derive(Debug, Clone, PartialEq)]
pub struct SlabReadout {
    /// 読めない欄と、その理由。並びは画面の欄の順
    pub errors: Vec<(SlabField, &'static str)>,
    /// クライテリアが 1 つも無い（変形角とたわみ絶対量が空欄で、表18-1 がオフ）
    pub no_criteria: bool,
    /// 計算はできるが、Fc が想定の範囲を外れている
    pub warning: Option<String>,
    pub shared: SharedRow,
    pub point: PointRow,
}

/// 共有値の行
#[derive(Debug, Clone, PartialEq)]
pub struct SharedRow {
    /// ヤング係数 E（N/mm²）
    pub young_modulus: String,
    /// 鉄筋コンクリートの単位体積重量 γ₁（kN/m³）
    pub rc_unit_weight: String,
    /// スラブ自重を除く荷重 wp（N/m²）
    pub wp: String,
}

/// 検討点の結果
#[derive(Debug, Clone, PartialEq)]
pub struct PointRow {
    /// 必要スラブ厚 t_req（mm）
    pub required_thickness: String,
    /// 決定条件の呼び名
    pub governing: String,
    /// 短辺スパン lx（m）
    pub short_span: String,
    /// 辺長比 λ
    pub aspect_ratio: String,
    /// 弾性たわみ δe（mm）
    pub elastic_deflection: String,
    /// 増大後たわみ（mm）
    pub amplified_deflection: String,
    /// 許容たわみ（mm）。たわみに効く条件が無ければ「—」
    pub allowable_deflection: String,
}

impl SlabReadout {
    pub fn from_inputs(inputs: &SlabInputs) -> Self {
        let mut errors = Vec::new();
        let mut read = |field: SlabField| match read_value(field, inputs.get(field)) {
            Ok(v) => v,
            Err(message) => {
                errors.push((field, message));
                None
            }
        };
        let [fc, finish, live, amplification, ratio, absolute] =
            SlabField::CONDITIONS.map(&mut read);
        let span_x = read(SlabField::SpanX);
        let span_y = read(SlabField::SpanY);
        let has_error = |field: SlabField| errors.iter().any(|(e, _)| *e == field);

        let criteria_readable =
            !has_error(SlabField::DeflectionRatio) && !has_error(SlabField::DeflectionAbsolute);
        let no_criteria =
            criteria_readable && ratio.is_none() && absolute.is_none() && !inputs.min_thickness;

        let concrete = fc.map(|fc| {
            Concrete::new(Stress::n_per_mm2(fc), ConcreteKind::Normal)
                .expect("Fc は read_value で正の有限値と見た")
        });
        let warning = concrete.and_then(|c| c.warning()).map(|w| w.to_string());
        let loads = finish
            .zip(live)
            .map(|(finish, live)| (AreaLoad::n_per_m2(finish), AreaLoad::n_per_m2(live)));
        // 共有値は Fc と荷重だけで決まる。ほかの欄が読めなくても出す
        let shared = concrete
            .zip(loads)
            .and_then(|(concrete, (finish, live))| shared_values(concrete, finish, live).ok());

        let evaluation = match (concrete, loads, amplification, span_x, span_y) {
            (
                Some(concrete),
                Some((finish_load, live_load)),
                Some(amplification),
                Some(x),
                Some(y),
            ) if errors.is_empty() && !no_criteria => {
                let conditions = Conditions {
                    concrete,
                    finish_load,
                    live_load,
                    amplification,
                    criteria: CriteriaSet {
                        deflection_ratio: ratio.map(|n| 1.0 / n),
                        deflection_absolute: absolute.map(Length::mm),
                        min_thickness: inputs.min_thickness,
                    },
                };
                let spans = Spans {
                    x: Length::mm(x * 1000.0),
                    y: Length::mm(y * 1000.0),
                };
                evaluate(&conditions, spans).ok()
            }
            _ => None,
        };

        Self {
            errors,
            no_criteria,
            warning,
            shared: SharedRow::new(shared),
            point: PointRow::new(evaluation),
        }
    }
}

impl SharedRow {
    fn new(shared: Option<SharedValues>) -> Self {
        match shared {
            None => Self {
                young_modulus: NOT_COMPUTED.into(),
                rc_unit_weight: NOT_COMPUTED.into(),
                wp: NOT_COMPUTED.into(),
            },
            Some(v) => Self {
                young_modulus: thousands(format!("{:.0}", v.young_modulus.as_n_per_mm2())),
                rc_unit_weight: format!("{:.1}", v.rc_unit_weight.as_kn_per_m3()),
                wp: thousands(format!("{:.0}", v.wp.as_n_per_m2())),
            },
        }
    }
}

impl PointRow {
    fn new(evaluation: Option<Evaluation>) -> Self {
        let Some(ev) = evaluation else {
            let dash = || NOT_COMPUTED.to_string();
            return Self {
                required_thickness: dash(),
                governing: dash(),
                short_span: dash(),
                aspect_ratio: dash(),
                elastic_deflection: dash(),
                amplified_deflection: dash(),
                allowable_deflection: dash(),
            };
        };
        let mm = |l: Length| format!("{:.2}", l.as_mm());
        Self {
            required_thickness: format!("{:.1}", ev.required_thickness.as_mm()),
            governing: criterion_name(ev.governing).into(),
            short_span: format!("{:.2}", ev.short_span.as_mm() / 1000.0),
            aspect_ratio: format!("{:.2}", ev.aspect_ratio),
            elastic_deflection: mm(ev.elastic_deflection),
            amplified_deflection: mm(ev.amplified_deflection),
            allowable_deflection: ev
                .allowable_deflection
                .map_or_else(|| NOT_COMPUTED.into(), mm),
        }
    }
}

/// 決定条件の呼び名
fn criterion_name(criterion: Criterion) -> &'static str {
    match criterion {
        Criterion::DeflectionRatio => "変形角",
        Criterion::DeflectionAbsolute => "たわみ絶対量",
        Criterion::MinThickness => "表18-1",
    }
}

/// `/tools/slab-thickness`。条件の Sheet と共有値、検討点の Sheet と結果の表を 1 つの列に置く（ADR-0006）。
/// 「入力」「結果」の部の見出しは置かず、表の見出しで区切る。
#[component]
pub(crate) fn SlabThicknessPage() -> Element {
    let inputs = use_signal(SlabInputs::default);
    let readout = SlabReadout::from_inputs(&inputs.read());
    let SharedRow {
        young_modulus,
        rc_unit_weight,
        wp,
    } = readout.shared.clone();
    let point = readout.point.clone();
    let condition_errors: Vec<_> = readout
        .errors
        .iter()
        .copied()
        .filter(|(f, _)| SlabField::CONDITIONS.contains(f))
        .collect();
    let span_errors: Vec<_> = readout
        .errors
        .iter()
        .copied()
        .filter(|(f, _)| !SlabField::CONDITIONS.contains(f))
        .collect();
    rsx! {
        Page { route: Route::ToolsSlabThickness {},
            Columns {
                Column {
                    // 入力のすぐ下に結果（`space-6`）。この組と次の組の間は Column の間（`space-7`）
                    div { class: "flex flex-col gap-6",
                        div { class: "flex flex-col gap-1",
                            ConditionSheet { inputs, errors: condition_errors.clone(), no_criteria: readout.no_criteria }
                            Notes {
                                li { "変形角とたわみ絶対量は空欄にすると判定に加えない" }
                                li { "変形増大係数は弾性たわみに乗じる" }
                            }
                            for (field , message) in condition_errors {
                                p { class: "text-label text-ng", "{field.label()}: {message}" }
                            }
                            if readout.no_criteria {
                                p { class: "text-label text-ng", {NO_CRITERIA} }
                            }
                            if let Some(warning) = readout.warning.clone() {
                                p { class: "text-label", "! {warning}" }
                            }
                        }
                        Block {
                            Caption { "共有値" }
                            Table { fixed: true,
                                thead {
                                    HeadRow {
                                        HeadCell { "E（N/mm²）" }
                                        HeadCell { "γ₁（kN/m³）" }
                                        HeadCell { "wp（N/m²）" }
                                    }
                                }
                                tbody {
                                    Row {
                                        NumCell { {young_modulus} }
                                        NumCell { {rc_unit_weight} }
                                        NumCell { {wp} }
                                    }
                                }
                            }
                            Notes {
                                li { "γ₁ は鉄筋コンクリートの単位体積重量" }
                                li { "wp は仕上げ荷重と積載荷重の和" }
                            }
                        }
                    }
                    div { class: "flex flex-col gap-6",
                        div { class: "flex flex-col gap-1",
                            PointSheet { inputs, errors: span_errors.clone() }
                            Notes {
                                li { "Lx・Ly は内法スパン" }
                            }
                            for (field , message) in span_errors {
                                p { class: "text-label text-ng", "{field.label()}: {message}" }
                            }
                        }
                        Block {
                            Caption { "必要スラブ厚" }
                            Table { fixed: true,
                                thead {
                                    HeadRow {
                                        HeadCell { "t_req（mm）" }
                                        HeadCell { left: true, "決定条件" }
                                    }
                                }
                                tbody {
                                    Row {
                                        NumCell { {point.required_thickness} }
                                        TextCell { {point.governing} }
                                    }
                                }
                            }
                        }
                    }
                    Block {
                        Caption { "短辺スパンと辺長比" }
                        Table { fixed: true,
                            thead {
                                HeadRow {
                                    HeadCell { "lx（m）" }
                                    HeadCell { "λ" }
                                }
                            }
                            tbody {
                                Row {
                                    NumCell { {point.short_span} }
                                    NumCell { {point.aspect_ratio} }
                                }
                            }
                        }
                        Notes {
                            li { "lx は Lx と Ly の小さいほう" }
                            li { "λ は長辺スパンと短辺スパンの比" }
                        }
                    }
                    Block {
                        Caption { "たわみ（mm）" }
                        Table { fixed: true,
                            thead {
                                HeadRow {
                                    HeadCell { "δe" }
                                    HeadCell { "増大後" }
                                    HeadCell { "許容" }
                                }
                            }
                            tbody {
                                Row {
                                    NumCell { {point.elastic_deflection} }
                                    NumCell { {point.amplified_deflection} }
                                    NumCell { {point.allowable_deflection} }
                                }
                            }
                        }
                        Notes {
                            li { "t_req での値" }
                            li { "δe は弾性たわみ、増大後は δe に変形増大係数を乗じた値" }
                            li { "許容は変形角とたわみ絶対量の上限の小さいほう" }
                        }
                    }
                }
            }
        }
    }
}

/// 条件の Sheet。欄を 1 行に 1 つずつ並べ、最後に表18-1 のチェック
#[component]
fn ConditionSheet(
    inputs: Signal<SlabInputs>,
    errors: Vec<(SlabField, &'static str)>,
    no_criteria: bool,
) -> Element {
    let has_error = |field: SlabField| errors.iter().any(|(e, _)| *e == field);
    // クライテリアが 1 つも無いときは、クライテリアの欄の見出しを `ng` にする
    let head_error = |field: SlabField| has_error(field) || (no_criteria && field.may_be_blank());
    rsx! {
        Sheet {
            tbody {
                for field in SlabField::CONDITIONS {
                    SheetRow {
                        SheetHead { label: field.label(), error: head_error(field) }
                        SheetCell {
                            SheetInput {
                                label: field.label(),
                                value: inputs.read().get(field).to_string(),
                                error: has_error(field),
                                oninput: move |e: FormEvent| inputs.write().set(field, e.value()),
                            }
                        }
                    }
                }
                SheetRow {
                    SheetHead { label: MIN_THICKNESS_LABEL, error: no_criteria }
                    SheetCell {
                        div { class: "flex h-6 items-center px-2",
                            input {
                                class: "size-4 accent-ink",
                                r#type: "checkbox",
                                "aria-label": MIN_THICKNESS_LABEL,
                                checked: inputs.read().min_thickness,
                                onchange: move |e| inputs.write().min_thickness = e.checked(),
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 検討点の Sheet。列見出しの行と、検討点 1 つの行
#[component]
fn PointSheet(inputs: Signal<SlabInputs>, errors: Vec<(SlabField, &'static str)>) -> Element {
    let has_error = |field: SlabField| errors.iter().any(|(e, _)| *e == field);
    rsx! {
        Sheet {
            thead {
                SheetHeadRow {
                    SheetHead { label: NAME_LABEL, error: false, scope: "col" }
                    for field in [SlabField::SpanX, SlabField::SpanY] {
                        SheetHead { label: field.label(), error: has_error(field), scope: "col" }
                    }
                }
            }
            tbody {
                SheetRow {
                    SheetCell {
                        SheetInput {
                            label: NAME_LABEL,
                            value: inputs.read().name.clone(),
                            error: false,
                            numeric: false,
                            oninput: move |e: FormEvent| inputs.write().name = e.value(),
                        }
                    }
                    for field in [SlabField::SpanX, SlabField::SpanY] {
                        SheetCell {
                            SheetInput {
                                label: field.label(),
                                value: inputs.read().get(field).to_string(),
                                error: has_error(field),
                                oninput: move |e: FormEvent| inputs.write().set(field, e.value()),
                            }
                        }
                    }
                }
            }
        }
    }
}
