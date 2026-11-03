//! Tool「必要スラブ厚さ」（`/tools/slab-thickness`）。言葉は `docs/tools/slab-thickness/GLOSSARY.md`。
//! 数値はすべて `rustruct-calc` の `shared_values`・`evaluate`・`iso_curves` から引き、ここでは入力の文字の
//! 読み取りと、表示の桁と図の範囲だけを決める。計算はブラウザの中で同期に回し、入力を変えるたびに結果と
//! 等厚曲線の図を描き直す。
//!
//! 今は検討点を 1 つだけ入れる。検討点のシートは後の Issue。

use dioxus::prelude::*;
use numerus::fixed;
use rustruct_calc::slab_thickness::{
    Conditions, CriteriaSet, Criterion, Evaluation, SharedValues, Spans, evaluate, shared_values,
};

mod figure;

pub use figure::SlabFigure;
use rustruct_types::concrete::{Concrete, ConcreteKind};
use rustruct_types::quantity::{AreaLoad, Length, Stress};

use crate::data::young_modulus;
use crate::inputs::{Definition, Field, InputGroup, Inputs, Kind, Layout, Reason, Sign};
use crate::table::{
    Block, Caption, Column, Columns, HeadCell, HeadRow, Notes, NumCell, Row, Table, TextCell,
};
use crate::{Page, Route};

/// 計算できない値の表示（`docs/ui-decisions.md`「エラーと警告」）
const NOT_COMPUTED: &str = "—";

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

    /// 名前と種類と初期値。初期値は移植元の slab のとおり（Issue #17）。荷重は 0 を受け、
    /// 変形角とたわみ絶対量は空欄を「判定に加えない」と読む
    fn definition(self) -> Definition {
        let number = Kind::number;
        let (name, kind) = match self {
            Self::Fc => ("Fc（N/mm²）", number(false, Sign::Positive, "24")),
            Self::FinishLoad => (
                "仕上げ荷重（N/m²）",
                number(false, Sign::NonNegative, "1000"),
            ),
            Self::LiveLoad => ("積載荷重（N/m²）", number(false, Sign::NonNegative, "1800")),
            Self::Amplification => ("変形増大係数", number(false, Sign::Positive, "16")),
            Self::DeflectionRatio => ("変形角 1/", number(true, Sign::Positive, "400")),
            Self::DeflectionAbsolute => ("たわみ絶対量（mm）", number(true, Sign::Positive, "")),
            Self::MinThickness => ("表18-1 最小厚さ", Kind::Check { initial: false }),
            Self::Name => ("名称", Kind::Text { initial: "S1" }),
            Self::SpanX => ("Lx（m）", number(false, Sign::Positive, "4.0")),
            Self::SpanY => ("Ly（m）", number(false, Sign::Positive, "5.0")),
        };
        Definition {
            name: name.into(),
            kind,
        }
    }
}

/// 入力の中身。読めるかは `SlabReadout` が決める
pub type SlabInputs = Inputs<SlabField>;

/// クライテリアが 1 つも無いときの文言
const NO_CRITERIA: Reason = "クライテリアを 1 つ以上入れる";

/// 入力から求めた表示。数値は丸めて文字にしたもの。計算できないときは「—」
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
        let read = SlabRead::from_inputs(inputs);
        let evaluation = read
            .conditions
            .zip(read.spans)
            .and_then(|(conditions, spans)| evaluate(&conditions, spans).ok());
        Self {
            errors: read.errors,
            no_criteria: read.no_criteria,
            warning: read.warning,
            shared: SharedRow::new(read.shared),
            point: PointRow::new(evaluation),
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
    /// 検討点の名称
    name: String,
}

impl SlabRead {
    fn from_inputs(inputs: &SlabInputs) -> Self {
        let mut reading = inputs.reading();
        let [fc, finish, live, amplification] = [
            SlabField::Fc,
            SlabField::FinishLoad,
            SlabField::LiveLoad,
            SlabField::Amplification,
        ]
        .map(|field| reading.number(field));
        let [ratio, absolute] = [SlabField::DeflectionRatio, SlabField::DeflectionAbsolute]
            .map(|field| reading.optional_number(field));
        let [span_x, span_y] =
            [SlabField::SpanX, SlabField::SpanY].map(|field| reading.number(field));
        let min_thickness = inputs.checked(SlabField::MinThickness);
        let errors = reading.errors();
        let has_error = |field: SlabField| errors.iter().any(|(e, _)| *e == field);

        let criteria_readable =
            !has_error(SlabField::DeflectionRatio) && !has_error(SlabField::DeflectionAbsolute);
        let no_criteria =
            criteria_readable && ratio.is_none() && absolute.is_none() && !min_thickness;

        let concrete = fc.map(|fc| {
            Concrete::new(Stress::n_per_mm2(fc), ConcreteKind::Normal)
                .expect("Fc は 0 より大きい有限の値と読んだ")
        });
        let warning = concrete.and_then(|c| c.warning()).map(|w| w.to_string());
        let loads = finish
            .zip(live)
            .map(|(finish, live)| (AreaLoad::n_per_m2(finish), AreaLoad::n_per_m2(live)));
        let shared = concrete
            .zip(loads)
            .and_then(|(concrete, (finish, live))| shared_values(concrete, finish, live).ok());

        let conditions_readable = !SlabField::on(SlabSheet::Conditions).any(has_error);
        let conditions = match (concrete, loads, amplification) {
            (Some(concrete), Some((finish_load, live_load)), Some(amplification))
                if conditions_readable && !no_criteria =>
            {
                Some(Conditions {
                    concrete,
                    finish_load,
                    live_load,
                    amplification,
                    criteria: CriteriaSet {
                        deflection_ratio: ratio.map(|n| 1.0 / n),
                        deflection_absolute: absolute.map(Length::mm),
                        min_thickness,
                    },
                })
            }
            _ => None,
        };
        let spans = span_x.zip(span_y).map(|(x, y)| Spans {
            x: Length::m(x),
            y: Length::m(y),
        });

        Self {
            errors,
            no_criteria,
            warning,
            shared,
            conditions,
            spans,
            name: inputs.value(SlabField::Name).trim().to_string(),
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
                young_modulus: young_modulus(v.young_modulus),
                rc_unit_weight: fixed(v.rc_unit_weight.as_kn_per_m3(), 1),
                wp: fixed(v.wp.as_n_per_m2(), 0),
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
        let mm = |l: Length| fixed(l.as_mm(), 2);
        Self {
            required_thickness: fixed(ev.required_thickness.as_mm(), 1),
            governing: criterion_name(ev.governing).into(),
            short_span: fixed(ev.short_span.as_m(), 2),
            aspect_ratio: fixed(ev.aspect_ratio, 2),
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
        vec![(SlabField::CRITERIA.to_vec(), NO_CRITERIA)]
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
                    InputGroup {
                        inputs,
                        layout: Layout::ColumnHeads(SlabField::on(SlabSheet::Point).collect()),
                        notes: vec!["Lx・Ly は内法スパン".into()],
                        errors: readout.errors.clone(),
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
