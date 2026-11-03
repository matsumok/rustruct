//! Tool「必要スラブ厚さ」（`/tools/slab-thickness`）。言葉は `docs/tools/slab-thickness/GLOSSARY.md`。
//! 数値はすべて `rustruct-calc` の `shared_values`・`evaluate`・`iso_curves` から引き、ここでは入力の文字の
//! 読み取りと、表示の桁と図の範囲だけを決める。計算はブラウザの中で同期に回し、入力を変えるたびに結果と
//! 等厚曲線の図を描き直す。
//!
//! 今は検討点を 1 つだけ入れる。検討点のシートは後の Issue。

use dioxus::prelude::*;
use numerus::fixed;
use rustruct_calc::slab_thickness::{
    Conditions, CriteriaSet, Criterion, Evaluation, InvalidInput, SharedValues, Spans, evaluate,
    shared_values,
};

mod figure;

pub use figure::SlabFigure;
use rustruct_types::concrete::{Concrete, ConcreteKind};
use rustruct_types::quantity::{AreaLoad, Length, Stress};

use crate::data::young_modulus;
use crate::inputs::{Definition, Field, InputGroup, Inputs, Kind, Layout, Reason};
use crate::table::{Block, Caption, Column, Columns, Notes, Table, TableColumn};
use crate::{Page, Route};

/// 欄。並びは条件の Sheet の行の順、検討点の Sheet の列の順
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabField {
    Fc,
    FinishLoad,
    LiveLoad,
    Amplification,
    /// 変形角の分母 n（1/n）
    DeflectionRatio,
    DeflectionAbsolute,
    /// 表18-1 のチェック
    MinThickness,
    /// 検討点の名称
    Name,
    SpanX,
    SpanY,
}

/// 欄の載る Sheet
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlabSheet {
    /// 条件。欄を 1 行に 1 つずつ、最後に表18-1 のチェック
    Conditions,
    /// 検討点。列見出しの行と、検討点 1 つの行
    Point,
}

impl SlabField {
    /// クライテリア。1 つも無いときに見出しを `ng` にする欄
    const CRITERIA: [Self; 3] = [
        Self::DeflectionRatio,
        Self::DeflectionAbsolute,
        Self::MinThickness,
    ];

    /// どの Sheet に載るか（網羅的な match なので、欄を足したときの決め忘れはコンパイルが止める）
    fn sheet(self) -> SlabSheet {
        match self {
            Self::Fc
            | Self::FinishLoad
            | Self::LiveLoad
            | Self::Amplification
            | Self::DeflectionRatio
            | Self::DeflectionAbsolute
            | Self::MinThickness => SlabSheet::Conditions,
            Self::Name | Self::SpanX | Self::SpanY => SlabSheet::Point,
        }
    }

    /// `sheet` に載る欄。`ALL` の順
    fn on(sheet: SlabSheet) -> impl Iterator<Item = Self> {
        Self::ALL
            .iter()
            .copied()
            .filter(move |f| f.sheet() == sheet)
    }
}

impl Field for SlabField {
    /// 条件の Sheet の行の順、検討点の Sheet の列の順
    const ALL: &'static [Self] = &[
        Self::Fc,
        Self::FinishLoad,
        Self::LiveLoad,
        Self::Amplification,
        Self::DeflectionRatio,
        Self::DeflectionAbsolute,
        Self::MinThickness,
        Self::Name,
        Self::SpanX,
        Self::SpanY,
    ];

    /// 名前と種類と初期値。初期値は移植元の slab のとおり（Issue #17）。変形角とたわみ絶対量は空欄を
    /// 「判定に加えない」と読む。値の規則は計算核の入口（`evaluate`）と、Fc は `Concrete` が見る
    fn definition(self) -> Definition {
        let number = Kind::number;
        let (name, kind) = match self {
            Self::Fc => ("Fc（N/mm²）", number(false, "24")),
            Self::FinishLoad => ("仕上げ荷重（N/m²）", number(false, "1000")),
            Self::LiveLoad => ("積載荷重（N/m²）", number(false, "1800")),
            Self::Amplification => ("変形増大係数", number(false, "16")),
            Self::DeflectionRatio => ("変形角 1/", number(true, "400")),
            Self::DeflectionAbsolute => ("たわみ絶対量（mm）", number(true, "")),
            Self::MinThickness => ("表18-1 最小厚さ", Kind::Check { initial: false }),
            Self::Name => ("名称", Kind::Text { initial: "S1" }),
            Self::SpanX => ("Lx（m）", number(false, "4.0")),
            Self::SpanY => ("Ly（m）", number(false, "5.0")),
        };
        Definition {
            name: name.into(),
            kind,
        }
    }
}

/// 入力の中身。読めるかは `SlabReadout` が決める
pub type SlabInputs = Inputs<SlabField>;

/// 計算核の入口が返す不正な欄を、画面の欄へ。クライテリアが 1 つも無いことは欄をまたぐので `None`
fn field_of(reason: InvalidInput) -> Option<SlabField> {
    match reason {
        InvalidInput::FinishLoad => Some(SlabField::FinishLoad),
        InvalidInput::LiveLoad => Some(SlabField::LiveLoad),
        InvalidInput::Amplification => Some(SlabField::Amplification),
        InvalidInput::DeflectionRatio => Some(SlabField::DeflectionRatio),
        InvalidInput::DeflectionAbsolute => Some(SlabField::DeflectionAbsolute),
        InvalidInput::NoCriteria => None,
        InvalidInput::SpanX => Some(SlabField::SpanX),
        InvalidInput::SpanY => Some(SlabField::SpanY),
    }
}

/// 入力から求めた表示。数値は丸めて文字にしたもの。計算できないときは無し（「—」）
#[derive(Debug, Clone, PartialEq)]
pub struct SlabReadout {
    /// 読めない欄と、その理由。並びは画面の欄の順
    pub errors: Vec<(SlabField, Reason)>,
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
    pub young_modulus: Option<String>,
    /// 鉄筋コンクリートの単位体積重量 γ₁（kN/m³）
    pub rc_unit_weight: Option<String>,
    /// スラブ自重を除く荷重 wp（N/m²）
    pub wp: Option<String>,
}

/// 検討点の結果
#[derive(Debug, Clone, PartialEq)]
pub struct PointRow {
    /// 必要スラブ厚 t_req（mm）
    pub required_thickness: Option<String>,
    /// 決定条件の呼び名
    pub governing: Option<String>,
    /// 短辺スパン lx（m）
    pub short_span: Option<String>,
    /// 辺長比 λ
    pub aspect_ratio: Option<String>,
    /// 弾性たわみ δe（mm）
    pub elastic_deflection: Option<String>,
    /// 増大後たわみ（mm）
    pub amplified_deflection: Option<String>,
    /// 許容たわみ（mm）。たわみに効く条件が無ければ無し
    pub allowable_deflection: Option<String>,
}

impl SlabReadout {
    pub fn from_inputs(inputs: &SlabInputs) -> Self {
        let read = SlabRead::from_inputs(inputs);
        Self {
            errors: read.errors,
            no_criteria: read.no_criteria,
            warning: read.warning,
            shared: SharedRow::new(read.shared),
            point: PointRow::new(read.evaluation),
        }
    }
}

/// 入力を読んだもの。結果の表と等厚曲線の図が同じ読み方を使う
struct SlabRead {
    errors: Vec<(SlabField, Reason)>,
    no_criteria: bool,
    warning: Option<String>,
    /// 共有値。Fc と荷重だけで決まるので、ほかの欄が読めなくても出す
    shared: Option<SharedValues>,
    /// 条件。条件の欄がすべて読め、クライテリアが 1 つ以上あるときだけ
    conditions: Option<Conditions>,
    /// 検討点のスパン。Lx・Ly が読めるときだけ
    spans: Option<Spans>,
    /// 検討点の評価。すべての欄が読めるときだけ
    evaluation: Option<Evaluation>,
    /// 検討点の名称
    name: String,
}

impl SlabRead {
    /// 読めない欄と、`Concrete` が受けない Fc は理由を集めて初期値で埋め（`Reading`）、計算核の入口が返す
    /// 不正な欄を足す
    fn from_inputs(inputs: &SlabInputs) -> Self {
        let mut reading = inputs.reading();
        let fc = reading.number(SlabField::Fc);
        let concrete = concrete(fc).unwrap_or_else(|| {
            reading.refuse(SlabField::Fc, "0 より大きい値を入れる");
            let initial = reading.initial(SlabField::Fc);
            concrete(initial.expect("Fc は空欄を受けない")).expect("Fc の初期値は正")
        });
        let [finish_load, live_load] = [SlabField::FinishLoad, SlabField::LiveLoad]
            .map(|field| AreaLoad::n_per_m2(reading.number(field)));
        let conditions = Conditions {
            concrete,
            finish_load,
            live_load,
            amplification: reading.number(SlabField::Amplification),
            criteria: CriteriaSet {
                deflection_ratio: reading
                    .optional_number(SlabField::DeflectionRatio)
                    .map(|n| 1.0 / n),
                deflection_absolute: reading
                    .optional_number(SlabField::DeflectionAbsolute)
                    .map(Length::mm),
                min_thickness: inputs.checked(SlabField::MinThickness),
            },
        };
        let [x, y] =
            [SlabField::SpanX, SlabField::SpanY].map(|field| Length::m(reading.number(field)));
        let spans = Spans { x, y };

        let evaluated = evaluate(&conditions, spans);
        let mut no_criteria = false;
        if let Err(invalid) = &evaluated {
            for &reason in invalid {
                match field_of(reason) {
                    Some(field) => reading.refuse(field, reason.to_string()),
                    None => no_criteria = true,
                }
            }
        }
        // 読めないクライテリアは入れてはあるので、「1 つも無い」とは言わない
        let no_criteria = no_criteria
            && !reading.refused(SlabField::DeflectionRatio)
            && !reading.refused(SlabField::DeflectionAbsolute);
        let errors = reading.errors();
        let has_error = |field: SlabField| errors.iter().any(|(e, _)| *e == field);

        let shared_fields = [SlabField::Fc, SlabField::FinishLoad, SlabField::LiveLoad];
        let shared = shared_values(concrete, finish_load, live_load)
            .ok()
            .filter(|_| !shared_fields.into_iter().any(has_error));
        let warning = if has_error(SlabField::Fc) {
            None
        } else {
            concrete.warning().map(|w| w.to_string())
        };
        let conditions_valid = !SlabField::on(SlabSheet::Conditions).any(has_error) && !no_criteria;
        let spans_valid = !has_error(SlabField::SpanX) && !has_error(SlabField::SpanY);

        Self {
            evaluation: evaluated.ok().filter(|_| errors.is_empty()),
            errors,
            no_criteria,
            warning,
            shared,
            conditions: conditions_valid.then_some(conditions),
            spans: spans_valid.then_some(spans),
            name: inputs.value(SlabField::Name).trim().to_string(),
        }
    }
}

/// Fc から普通コンクリート。`Concrete` が受けない Fc は `None`
fn concrete(fc: f64) -> Option<Concrete> {
    Concrete::new(Stress::n_per_mm2(fc), ConcreteKind::Normal).ok()
}

impl SharedRow {
    fn new(shared: Option<SharedValues>) -> Self {
        match shared {
            None => Self {
                young_modulus: None,
                rc_unit_weight: None,
                wp: None,
            },
            Some(v) => Self {
                young_modulus: Some(young_modulus(v.young_modulus)),
                rc_unit_weight: Some(fixed(v.rc_unit_weight.as_kn_per_m3(), 1)),
                wp: Some(fixed(v.wp.as_n_per_m2(), 0)),
            },
        }
    }
}

impl PointRow {
    fn new(evaluation: Option<Evaluation>) -> Self {
        let Some(ev) = evaluation else {
            return Self {
                required_thickness: None,
                governing: None,
                short_span: None,
                aspect_ratio: None,
                elastic_deflection: None,
                amplified_deflection: None,
                allowable_deflection: None,
            };
        };
        let mm = |l: Length| fixed(l.as_mm(), 2);
        Self {
            required_thickness: Some(fixed(ev.required_thickness.as_mm(), 1)),
            governing: Some(criterion_name(ev.governing).into()),
            short_span: Some(fixed(ev.short_span.as_m(), 2)),
            aspect_ratio: Some(fixed(ev.aspect_ratio, 2)),
            elastic_deflection: Some(mm(ev.elastic_deflection)),
            amplified_deflection: Some(mm(ev.amplified_deflection)),
            allowable_deflection: ev.allowable_deflection.map(mm),
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

/// `/tools/slab-thickness`。条件の Sheet と共有値、検討点の Sheet と結果の表を置いた列と、等厚曲線の図の列
/// （ADR site-0006）。「入力」「結果」の部の見出しは置かず、表の見出しで区切る。
#[component]
pub(crate) fn SlabThicknessPage() -> Element {
    let inputs = use_signal(SlabInputs::default);
    let readout = SlabReadout::from_inputs(&inputs.read());
    let figure = use_memo(move || SlabFigure::from_inputs(&inputs.read()));
    let SharedRow {
        young_modulus,
        rc_unit_weight,
        wp,
    } = readout.shared.clone();
    let point = readout.point.clone();
    let across = if readout.no_criteria {
        vec![(
            SlabField::CRITERIA.to_vec(),
            InvalidInput::NoCriteria.to_string(),
        )]
    } else {
        Vec::new()
    };
    rsx! {
        Page { route: Route::ToolsSlabThickness {},
            Columns {
                Column {
                    InputGroup {
                        inputs,
                        layout: Layout::RowHeads(SlabField::on(SlabSheet::Conditions).map(|field| vec![field]).collect()),
                        notes: vec![
                            "変形角とたわみ絶対量は空欄にすると判定に加えない".into(),
                            "変形増大係数は弾性たわみに乗じる".into(),
                        ],
                        errors: readout.errors.clone(),
                        across,
                        warning: readout.warning.clone(),
                        Block {
                            Caption { "共有値" }
                            Table {
                                fixed: true,
                                columns: vec![
                                    TableColumn::number("E").unit("N/mm²"),
                                    TableColumn::number("γ₁").unit("kN/m³"),
                                    TableColumn::number("wp").unit("N/m²"),
                                ],
                                rows: vec![vec![young_modulus.into(), rc_unit_weight.into(), wp.into()]],
                            }
                            Notes {
                                li { "γ₁ は鉄筋コンクリートの単位体積重量" }
                                li { "wp は仕上げ荷重と積載荷重の和" }
                            }
                        }
                    }
                    InputGroup {
                        inputs,
                        layout: Layout::ColumnHeads(SlabField::on(SlabSheet::Point).collect()),
                        notes: vec!["Lx・Ly は内法スパン".into()],
                        errors: readout.errors.clone(),
                        Block {
                            Caption { "必要スラブ厚" }
                            Table {
                                fixed: true,
                                columns: vec![
                                    TableColumn::number("t_req").unit("mm"),
                                    TableColumn::name("決定条件"),
                                ],
                                rows: vec![vec![point.required_thickness.into(), point.governing.into()]],
                            }
                        }
                    }
                    Block {
                        Caption { "短辺スパンと辺長比" }
                        Table {
                            fixed: true,
                            columns: vec![TableColumn::number("lx").unit("m"), TableColumn::number("λ")],
                            rows: vec![vec![point.short_span.into(), point.aspect_ratio.into()]],
                        }
                        Notes {
                            li { "lx は Lx と Ly の小さいほう" }
                            li { "λ は長辺スパンと短辺スパンの比" }
                        }
                    }
                    Block {
                        Caption { "たわみ（mm）" }
                        Table {
                            fixed: true,
                            columns: vec![
                                TableColumn::number("δe"),
                                TableColumn::number("増大後"),
                                TableColumn::number("許容"),
                            ],
                            rows: vec![
                                vec![
                                    point.elastic_deflection.into(),
                                    point.amplified_deflection.into(),
                                    point.allowable_deflection.into(),
                                ],
                            ],
                        }
                        Notes {
                            li { "t_req での値" }
                            li { "δe は弾性たわみ、増大後は δe に変形増大係数を乗じた値" }
                            li { "許容は変形角とたわみ絶対量の上限の小さいほう" }
                        }
                    }
                }
                Column {
                    FigureBlock { figure: figure() }
                }
            }
        }
    }
}

/// 等厚曲線の図の一まとまり。図の名前、図、読み方の注記。条件が読めない間やクライテリアが無いときは
/// 図の代わりに注記だけを置く（理由は条件の Sheet の下が言う）
#[component]
fn FigureBlock(figure: Option<SlabFigure>) -> Element {
    rsx! {
        Block {
            Caption { "必要スラブ厚の等厚曲線（mm）" }
            match figure {
                Some(figure) => rsx! {
                    figure { role: "img", "aria-label": figure.label,
                        div { class: "dark:hidden", dangerous_inner_html: figure.light_svg }
                        div { class: "hidden dark:block", dangerous_inner_html: figure.dark_svg }
                    }
                    Notes {
                        li { "曲線は 10 mm ごと、色は 20 mm ごと" }
                        li { "80 mm 未満は塗らない" }
                        li { "丸の印は検討点" }
                        li { "左上は条件の要約。α は変形増大係数、δ は増大後たわみ" }
                    }
                },
                None => rsx! {
                    Notes {
                        li { "条件が読めるまで図は描かない" }
                    }
                },
            }
        }
    }
}
