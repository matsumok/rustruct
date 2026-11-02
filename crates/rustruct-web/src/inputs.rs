//! 入力の組（`GLOSSARY.md`「欄」「入力の組」）。決め事は `docs/ui-decisions.md`「入力と表示の区別」「エラーと警告」。
//!
//! ページは欄の enum に [`Field`] を実装して、欄ごとの名前・種類・初期値を宣言する。値は [`Inputs`] が
//! 打ったままの文字で持ち、種類に応じた型で読む。読み取りの規則は 5 つ（空欄、3 桁区切りの「,」、0 と負、
//! 有限でない数、選択肢に無い記号）、読めない理由の文言は 4 つ（「値を入れる」「数値で入れる」
//! 「0 以上の値を入れる」「0 より大きい値を入れる」）で、crate は使わない。
//! 描くのは [`InputGroup`]: Sheet → 注記 → 読めない欄の理由 → 警告の順に `space-1` で積み、
//! その下に結果の表を `space-6` 空けて置く。ng の印（見出しと欄の枠）と `aria-label` もここが付ける。

use std::marker::PhantomData;

use dioxus::prelude::*;

use crate::sheet::{
    Sheet, SheetCell, SheetCheck, SheetHead, SheetHeadRow, SheetInput, SheetRow, SheetSelect,
};
use crate::table::Notes;

/// 読めない欄の理由
pub type Reason = &'static str;

/// ページの欄の enum が実装する。`ALL` は欄の並び（値の置き場の順）
pub trait Field: Copy + Eq + std::fmt::Debug + 'static {
    const ALL: &'static [Self];

    /// 欄の定義。1 つの match で返す
    fn definition(self) -> Definition;
}

/// 欄の定義
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    /// 欄の名前。単位は括弧で名前に付ける（「Lx（m）」）
    pub name: String,
    pub kind: Kind,
}

/// 欄の種類と、種類ごとの規則と初期値
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Number {
        /// 空欄を受ける（空欄は「無し」と読む）
        blank: bool,
        sign: Sign,
        initial: &'static str,
    },
    Text {
        initial: &'static str,
    },
    Check {
        initial: bool,
    },
    Choice {
        /// 選択肢の記号と名前。並びは選択肢の順
        options: Vec<(char, &'static str)>,
        initial: char,
    },
}

/// 数値の欄が受ける符号
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// 0 より大きい値だけ
    Positive,
    /// 0 以上
    NonNegative,
    /// 負も（上向きの荷重など）
    Any,
}

impl Kind {
    /// 数値の欄
    pub const fn number(blank: bool, sign: Sign, initial: &'static str) -> Self {
        Self::Number {
            blank,
            sign,
            initial,
        }
    }

    /// 置き場に入れる初期値の文字
    fn initial(&self) -> String {
        match self {
            Self::Number { initial, .. } | Self::Text { initial } => (*initial).into(),
            Self::Check { initial } => initial.to_string(),
            Self::Choice { initial, .. } => (*initial).into(),
        }
    }

    /// チェックと選択が受ける文字。数値と文字は何でも受ける
    fn accepts(&self, value: &str) -> bool {
        match self {
            Self::Number { .. } | Self::Text { .. } => true,
            Self::Check { .. } => value.parse::<bool>().is_ok(),
            Self::Choice { options, .. } => options
                .iter()
                .any(|(symbol, _)| symbol.to_string() == value),
        }
    }
}

/// 値の置き場。欄ごとに打ったままの文字を持つ
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inputs<F: Field> {
    /// `F::ALL` の順
    values: Vec<String>,
    field: PhantomData<F>,
}

impl<F: Field> Default for Inputs<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Field> Inputs<F> {
    /// 初期値で埋める
    pub fn new() -> Self {
        Self {
            values: F::ALL
                .iter()
                .map(|field| field.definition().kind.initial())
                .collect(),
            field: PhantomData,
        }
    }

    fn index(field: F) -> usize {
        F::ALL
            .iter()
            .position(|f| *f == field)
            .expect("欄は Field::ALL に載っている")
    }

    /// 打ったままの文字。文字の欄の読み取りもこれ
    pub fn value(&self, field: F) -> &str {
        &self.values[Self::index(field)]
    }

    /// 欄に文字を入れる。チェックは `true` / `false`、選択は選択肢の記号だけを受け、ほかの文字では変えない
    pub fn set(&mut self, field: F, value: impl Into<String>) {
        let value = value.into();
        if field.definition().kind.accepts(&value) {
            self.values[Self::index(field)] = value;
        }
    }

    /// 数値の欄。空欄を受ける欄の空欄は `None`。3 桁区切りの「,」は読み飛ばす
    pub fn optional_number(&self, field: F) -> Result<Option<f64>, Reason> {
        let Kind::Number { blank, sign, .. } = field.definition().kind else {
            panic!("{field:?} は数値の欄でない");
        };
        let text = self.value(field).trim().replace(',', "");
        if text.is_empty() {
            return if blank {
                Ok(None)
            } else {
                Err("値を入れる")
            };
        }
        let value: f64 = text
            .parse()
            .ok()
            .filter(|v: &f64| v.is_finite())
            .ok_or("数値で入れる")?;
        match sign {
            Sign::Positive if value <= 0.0 => Err("0 より大きい値を入れる"),
            Sign::NonNegative if value < 0.0 => Err("0 以上の値を入れる"),
            _ => Ok(Some(value)),
        }
    }

    /// 空欄を受けない数値の欄
    pub fn number(&self, field: F) -> Result<f64, Reason> {
        self.optional_number(field)
            .map(|v| v.unwrap_or_else(|| panic!("{field:?} は空欄を受ける欄")))
    }

    /// チェックの欄
    pub fn checked(&self, field: F) -> bool {
        self.value(field) == "true"
    }

    /// 選択の欄。選択肢の記号
    pub fn choice(&self, field: F) -> char {
        self.value(field)
            .chars()
            .next()
            .expect("選択の欄は記号を 1 つ持つ")
    }

    /// 欄を順に読み、読めない欄と理由を集める
    pub fn reading(&self) -> Reading<'_, F> {
        Reading {
            inputs: self,
            errors: Vec::new(),
        }
    }
}

/// 欄を順に読んだ記録。読めない欄は読んだ順に理由と並ぶ
pub struct Reading<'a, F: Field> {
    inputs: &'a Inputs<F>,
    errors: Vec<(F, Reason)>,
}

impl<F: Field> Reading<'_, F> {
    /// 空欄を受けない数値の欄。読めなければ理由を集めて `None`
    pub fn number(&mut self, field: F) -> Option<f64> {
        let result = self.inputs.number(field);
        self.collect(field, result)
    }

    /// 数値の欄。空欄も読めない欄も `None`（読めない欄は理由を集める）
    pub fn optional_number(&mut self, field: F) -> Option<f64> {
        let result = self.inputs.optional_number(field);
        self.collect(field, result).flatten()
    }

    fn collect<T>(&mut self, field: F, result: Result<T, Reason>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(reason) => {
                self.refuse(field, reason);
                None
            }
        }
    }

    /// ページが決める理由（欄の規則の外の範囲など）を足す
    pub fn refuse(&mut self, field: F, reason: Reason) {
        self.errors.push((field, reason));
    }

    pub fn errors(self) -> Vec<(F, Reason)> {
        self.errors
    }
}

/// Sheet の並べ方（`docs/ui-decisions.md`「入力と表示の区別」）
#[derive(Debug, Clone, PartialEq)]
pub enum Layout<F: Field> {
    /// 行見出し: 左に見出し、右に欄。行ごとに 1 欄か 2 組
    RowHeads(Vec<Vec<F>>),
    /// 列見出し: 上に見出しの行、下に値の行
    ColumnHeads(Vec<F>),
}

impl<F: Field> Layout<F> {
    /// 欄の順（読めない欄の理由を並べる順）
    fn fields(&self) -> Vec<F> {
        match self {
            Self::RowHeads(rows) => rows.iter().flatten().copied().collect(),
            Self::ColumnHeads(fields) => fields.clone(),
        }
    }
}

/// 入力の組。Sheet、注記、読めない欄の理由（「欄の名前: 理由」を欄の順に、件数なし）、欄をまたぐ
/// エラーの文言、警告（「! …」）の順に `space-1` で積み、子（結果の表）をその下に `space-6` 空けて置く。
/// `errors` はこの Sheet に無い欄のものも受け、自分の欄のものだけを出す。`across` は関わる欄の見出しを `ng` にする
#[component]
pub(crate) fn InputGroup<F: Field>(
    inputs: Signal<Inputs<F>>,
    layout: Layout<F>,
    #[props(default)] notes: Vec<String>,
    #[props(default)] errors: Vec<(F, Reason)>,
    #[props(default)] across: Vec<(Vec<F>, Reason)>,
    #[props(default)] warning: Option<String>,
    children: Element,
) -> Element {
    let own: Vec<(F, Reason)> = layout
        .fields()
        .into_iter()
        .flat_map(|field| errors.iter().filter(move |(f, _)| *f == field).copied())
        .collect();
    let has_error = |field: F| own.iter().any(|(f, _)| *f == field);
    let head_error =
        |field: F| has_error(field) || across.iter().any(|(fields, _)| fields.contains(&field));
    rsx! {
        div { class: "flex flex-col gap-6",
            div { class: "flex flex-col gap-1",
                Sheet {
                    match &layout {
                        Layout::RowHeads(rows) => rsx! {
                            tbody {
                                for row in rows.iter() {
                                    SheetRow {
                                        for field in row.iter().copied() {
                                            SheetHead { label: field.definition().name, error: head_error(field) }
                                            SheetCell {
                                                Cell { inputs, field, error: has_error(field) }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Layout::ColumnHeads(fields) => rsx! {
                            thead {
                                SheetHeadRow {
                                    for field in fields.iter().copied() {
                                        SheetHead { label: field.definition().name, error: head_error(field), scope: "col" }
                                    }
                                }
                            }
                            tbody {
                                SheetRow {
                                    for field in fields.iter().copied() {
                                        SheetCell {
                                            Cell { inputs, field, error: has_error(field) }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }
                if !notes.is_empty() {
                    Notes {
                        for note in notes {
                            li { {note} }
                        }
                    }
                }
                for (field , reason) in own.iter().copied() {
                    p { class: "text-label text-ng", "{field.definition().name}: {reason}" }
                }
                for (_ , message) in across.iter() {
                    p { class: "text-label text-ng", {*message} }
                }
                if let Some(warning) = warning {
                    p { class: "text-label", "! {warning}" }
                }
            }
            {children}
        }
    }
}

/// 1 つの欄。種類で見た目を選び、`aria-label` に欄の名前を付ける
#[component]
fn Cell<F: Field>(inputs: Signal<Inputs<F>>, field: F, error: bool) -> Element {
    let Definition { name, kind } = field.definition();
    match kind {
        Kind::Number { .. } | Kind::Text { .. } => rsx! {
            SheetInput {
                label: name,
                value: inputs.read().value(field).to_string(),
                error,
                numeric: matches!(kind, Kind::Number { .. }),
                oninput: move |e: FormEvent| inputs.write().set(field, e.value()),
            }
        },
        Kind::Check { .. } => rsx! {
            SheetCheck {
                label: name,
                checked: inputs.read().checked(field),
                onchange: move |e: FormEvent| inputs.write().set(field, e.checked().to_string()),
            }
        },
        Kind::Choice { options, .. } => rsx! {
            SheetSelect {
                label: name,
                options,
                selected: inputs.read().choice(field),
                onchange: move |e: FormEvent| inputs.write().set(field, e.value()),
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4 種類の欄を 1 つずつ持つ試験用の欄
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Test {
        /// 0 より大きい数値
        Positive,
        /// 0 以上の数値
        Load,
        /// 負も受ける数値
        Pressure,
        /// 空欄を受ける、0 より大きい数値
        Ratio,
        Name,
        Table,
        Support,
    }

    impl Field for Test {
        const ALL: &'static [Self] = &[
            Self::Positive,
            Self::Load,
            Self::Pressure,
            Self::Ratio,
            Self::Name,
            Self::Table,
            Self::Support,
        ];

        fn definition(self) -> Definition {
            let (name, kind) = match self {
                Self::Positive => (
                    "Lx（m）",
                    Kind::Number {
                        blank: false,
                        sign: Sign::Positive,
                        initial: "4.0",
                    },
                ),
                Self::Load => (
                    "積載荷重（N/m²）",
                    Kind::Number {
                        blank: false,
                        sign: Sign::NonNegative,
                        initial: "1,800",
                    },
                ),
                Self::Pressure => (
                    "p（kN/m²）",
                    Kind::Number {
                        blank: false,
                        sign: Sign::Any,
                        initial: "10",
                    },
                ),
                Self::Ratio => (
                    "変形角 1/",
                    Kind::Number {
                        blank: true,
                        sign: Sign::Positive,
                        initial: "",
                    },
                ),
                Self::Name => ("名称", Kind::Text { initial: "S1" }),
                Self::Table => ("表18-1 最小厚さ", Kind::Check { initial: false }),
                Self::Support => (
                    "x = 0（左）",
                    Kind::Choice {
                        options: vec![('F', "固定"), ('S', "単純支持"), ('R', "自由")],
                        initial: 'F',
                    },
                ),
            };
            Definition {
                name: name.into(),
                kind,
            }
        }
    }

    #[test]
    fn new_inputs_hold_the_initial_value_of_every_field() {
        let inputs = Inputs::<Test>::new();
        assert_eq!(inputs.value(Test::Positive), "4.0");
        assert_eq!(inputs.value(Test::Load), "1,800");
        assert_eq!(inputs.value(Test::Ratio), "");
        assert_eq!(inputs.value(Test::Name), "S1");
        assert!(!inputs.checked(Test::Table));
        assert_eq!(inputs.choice(Test::Support), 'F');
        assert_eq!(inputs, Inputs::default());
    }

    #[test]
    fn a_number_is_read_without_the_thousands_separator_and_the_surrounding_spaces() {
        let mut inputs = Inputs::<Test>::new();
        assert_eq!(inputs.number(Test::Load), Ok(1800.0));
        inputs.set(Test::Positive, " 12,345.5 ");
        assert_eq!(inputs.number(Test::Positive), Ok(12345.5));
    }

    #[test]
    fn a_blank_or_unparsable_number_names_its_reason() {
        let mut inputs = Inputs::<Test>::new();
        inputs.set(Test::Positive, "  ");
        assert_eq!(inputs.number(Test::Positive), Err("値を入れる"));
        for text in ["abc", "inf", "NaN", "1e400", "1,2,3x"] {
            inputs.set(Test::Positive, text);
            assert_eq!(inputs.number(Test::Positive), Err("数値で入れる"), "{text}");
        }
    }

    #[test]
    fn zero_is_refused_or_accepted_by_the_definition() {
        let mut inputs = Inputs::<Test>::new();
        inputs.set(Test::Positive, "0");
        assert_eq!(inputs.number(Test::Positive), Err("0 より大きい値を入れる"));
        inputs.set(Test::Positive, "-1");
        assert_eq!(inputs.number(Test::Positive), Err("0 より大きい値を入れる"));
        inputs.set(Test::Load, "0");
        assert_eq!(inputs.number(Test::Load), Ok(0.0));
        inputs.set(Test::Load, "-0.5");
        assert_eq!(inputs.number(Test::Load), Err("0 以上の値を入れる"));
        inputs.set(Test::Pressure, "-10");
        assert_eq!(inputs.number(Test::Pressure), Ok(-10.0));
    }

    #[test]
    fn a_field_that_accepts_a_blank_reads_the_blank_as_none() {
        let mut inputs = Inputs::<Test>::new();
        assert_eq!(inputs.optional_number(Test::Ratio), Ok(None));
        inputs.set(Test::Ratio, "400");
        assert_eq!(inputs.optional_number(Test::Ratio), Ok(Some(400.0)));
        inputs.set(Test::Ratio, "0");
        assert_eq!(
            inputs.optional_number(Test::Ratio),
            Err("0 より大きい値を入れる")
        );
        // 空欄を受けない欄も同じ読み方で、空欄だけが理由になる
        assert_eq!(inputs.optional_number(Test::Load), Ok(Some(1800.0)));
        inputs.set(Test::Load, "");
        assert_eq!(inputs.optional_number(Test::Load), Err("値を入れる"));
    }

    #[test]
    fn a_check_and_a_choice_take_only_their_own_symbols() {
        let mut inputs = Inputs::<Test>::new();
        inputs.set(Test::Table, "true");
        assert!(inputs.checked(Test::Table));
        inputs.set(Test::Table, "yes");
        assert!(inputs.checked(Test::Table), "知らない文字で変わった");
        inputs.set(Test::Table, "false");
        assert!(!inputs.checked(Test::Table));

        inputs.set(Test::Support, 'R');
        assert_eq!(inputs.choice(Test::Support), 'R');
        inputs.set(Test::Support, "Z");
        assert_eq!(
            inputs.choice(Test::Support),
            'R',
            "選択肢に無い記号で変わった"
        );
        inputs.set(Test::Support, "");
        assert_eq!(inputs.choice(Test::Support), 'R');
    }

    /// `InputGroup` を 1 つ置いたページ。読めない欄を欄の順でなく渡し、Sheet に無い欄の理由も混ぜる
    #[component]
    fn GroupPage() -> Element {
        let inputs = use_signal(Inputs::<Test>::new);
        rsx! {
            InputGroup {
                inputs,
                layout: Layout::RowHeads(vec![
                    vec![Test::Positive, Test::Load],
                    vec![Test::Ratio],
                    vec![Test::Table, Test::Support],
                ]),
                notes: vec!["注記".into()],
                errors: vec![
                    (Test::Load, "値を入れる"),
                    (Test::Positive, "数値で入れる"),
                    (Test::Name, "この Sheet に無い欄"),
                ],
                across: vec![(vec![Test::Ratio, Test::Table], "どれか 1 つ")],
                warning: Some("想定の外".into()),
                p { "結果の表" }
            }
        }
    }

    #[test]
    fn the_group_stacks_the_sheet_notes_reasons_message_and_warning_before_its_children() {
        let mut dom = VirtualDom::new(GroupPage);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let text = html.replace("<", " <");
        let mut from = 0;
        for needle in [
            "Lx（m）",
            "積載荷重（N/m²）",
            "変形角 1/",
            "表18-1 最小厚さ",
            "x = 0（左）",
            "固定",
            "注記",
            "Lx（m）: 数値で入れる",
            "積載荷重（N/m²）: 値を入れる",
            "どれか 1 つ",
            "! 想定の外",
            "結果の表",
        ] {
            let at = text[from..]
                .find(needle)
                .unwrap_or_else(|| panic!("「{needle}」が無い（またはこの順でない）: {html}"));
            from += at + needle.len();
        }
        assert!(!html.contains("この Sheet に無い欄"), "{html}");
        // ng の文字は、読めない 2 欄とまたぐエラーの 2 欄の見出し、理由 2 行とまたぐエラーの文言。
        // ng の枠は読めない 2 欄だけ（またぐエラーの欄は読めている）
        assert_eq!(html.matches("text-ng").count(), 4 + 3, "{html}");
        assert_eq!(html.matches("outline-ng").count(), 2, "{html}");
        for label in ["Lx（m）", "表18-1 最小厚さ", "x = 0（左）"] {
            assert!(html.contains(&format!("aria-label=\"{label}\"")), "{html}");
        }
        assert!(html.contains("type=\"checkbox\""), "{html}");
        assert!(html.contains("<select"), "{html}");
    }

    #[test]
    fn a_reading_collects_the_unreadable_fields_in_the_order_they_are_read() {
        let mut inputs = Inputs::<Test>::new();
        inputs.set(Test::Positive, "x");
        inputs.set(Test::Ratio, "-1");
        let mut reading = inputs.reading();
        let ratio = reading.optional_number(Test::Ratio);
        let positive = reading.number(Test::Positive);
        let load = reading.number(Test::Load);
        reading.refuse(Test::Load, "0 以上 0.5 未満で入れる");
        assert_eq!((ratio, positive, load), (None, None, Some(1800.0)));
        assert_eq!(
            reading.errors(),
            vec![
                (Test::Ratio, "0 より大きい値を入れる"),
                (Test::Positive, "数値で入れる"),
                (Test::Load, "0 以上 0.5 未満で入れる"),
            ]
        );
    }
}
