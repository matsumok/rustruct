//! 入力の組（`GLOSSARY.md`「欄」「入力の組」）。決め事は `docs/ui-decisions.md`「入力と表示の区別」「エラーと警告」。
//!
//! ページは欄の enum に [`Field`] を実装して、欄ごとの名前・種類・初期値を宣言する。値は [`Inputs`] が
//! 打ったままの文字で持ち、ページは [`Reading`] を通して種類に応じた型で読む。読み取りの規則は文字の話の
//! 4 つ（空欄、3 桁区切りの「,」、有限でない数、選択肢に無い記号）、読めない理由の文言は 2 つ（「値を入れる」
//! 「数値で入れる」）で、crate は使わない。0 や負を受けるかの値の規則は計算核の入口と計算核の型（ν、Fc など）が
//! 持つ。型が受けない値は [`Reading::typed`] が、入口が返す理由は [`Reading::invalid`] が欄に写す。
//! 読み取りの結果は [`InputRecord`]（入力の組の記録）の 1 つの値で、計算の結果が持ち、入力の組にそのまま渡る。
//! 描くのは [`InputGroup`]: Sheet → 注記 → 読めない欄の理由 → 欄をまたぐ理由 → 警告の順に `space-1` で積み、
//! その下に結果の表を `space-6` 空けて置く。ng の印（見出しと欄の枠）と `aria-label` もここが付ける。
//! 読むだけの欄（[`Kind::ReadOnly`]）は書けず、出す値はページが読み取りの中で決めて記録に持たせる（[`Reading::show`]）。

use std::marker::PhantomData;

use dioxus::prelude::*;

use crate::sheet::{
    Sheet, SheetBlank, SheetCell, SheetCheck, SheetHead, SheetHeadRow, SheetInput, SheetRow,
    SheetSelect, SheetValue,
};
use crate::table::{NO_VALUE, Notes};

/// 読めない欄の理由。文字の規則の文言か、計算核の入口が返す理由の文言
pub type Reason = String;

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
        /// 読める値。読めない欄の埋め草にもなる（[`Reading`]）
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
    /// 読むだけの欄。書けず、値の置き場も使わない。出す値はページが読み取りの中で決める（[`Reading::show`]）
    ReadOnly,
}

impl Kind {
    /// 数値の欄
    pub const fn number(blank: bool, initial: &'static str) -> Self {
        Self::Number { blank, initial }
    }

    /// 置き場に入れる初期値の文字
    fn initial(&self) -> String {
        match self {
            Self::Number { initial, .. } | Self::Text { initial } => (*initial).into(),
            Self::Check { initial } => initial.to_string(),
            Self::Choice { initial, .. } => (*initial).into(),
            Self::ReadOnly => String::new(),
        }
    }

    /// チェックと選択が受ける文字。数値と文字は何でも受け、読むだけの欄は何も受けない
    fn accepts(&self, value: &str) -> bool {
        match self {
            Self::ReadOnly => false,
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

    /// 数値の欄。空欄を受ける欄の空欄は `None`。3 桁区切りの「,」は読み飛ばす。0 や負も読む
    /// （値の規則は計算核の入口が見る）
    pub fn optional_number(&self, field: F) -> Result<Option<f64>, Reason> {
        let Kind::Number { blank, .. } = field.definition().kind else {
            panic!("{field:?} は数値の欄でない");
        };
        read_number(self.value(field), blank).map_err(Reason::from)
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

    /// 欄を順に読み、入力の組の記録を集める
    pub fn reading(&self) -> Reading<'_, F> {
        Reading {
            inputs: self,
            reasons: Vec::new(),
            across: Vec::new(),
            warnings: Vec::new(),
            shown: Vec::new(),
        }
    }
}

/// ページの値の置き場の signal。初めは初期値で、描く前に根の context に `Inputs<F>` があればそれで始める
/// （打った後の状態の HTML をテストで描くため。`render_path_with`）
pub(crate) fn use_inputs<F: Field>() -> Signal<Inputs<F>> {
    use_signal(|| try_consume_context::<Inputs<F>>().unwrap_or_default())
}

/// 数値の文字を読む。空欄は `blank` なら `None`。読めなければ理由の文言
fn read_number(text: &str, blank: bool) -> Result<Option<f64>, &'static str> {
    let text = text.trim().replace(',', "");
    if text.is_empty() {
        return if blank {
            Ok(None)
        } else {
            Err("値を入れる")
        };
    }
    text.parse()
        .ok()
        .filter(|v: &f64| v.is_finite())
        .map(Some)
        .ok_or("数値で入れる")
}

/// 欄を読む。ページの `from_inputs` は欄をすべてここから読み、[`Self::finish`] で入力の組の記録にする。
///
/// 読めない欄は、理由を集めたうえで欄の初期値で埋める。読めても計算核の型が受けない値（ν、Fc など）は
/// [`Self::typed`] が理由を足して初期値の型で埋める。ページは埋めた値ごと計算核の入口を呼び、読めた欄の値の
/// 規則を確かめさせて（読めない欄と読めても不正な欄をいっぺんに出すため）、返った理由を [`Self::invalid`] で
/// 足す。理由が 1 つでもあるあいだ、その結果は使わない
pub struct Reading<'a, F: Field> {
    inputs: &'a Inputs<F>,
    reasons: Vec<(F, Reason)>,
    across: Vec<(Vec<F>, Reason)>,
    warnings: Vec<(Vec<F>, String)>,
    shown: Vec<(F, String)>,
}

impl<'a, F: Field> Reading<'a, F> {
    /// 空欄を受けない数値の欄。読めなければ理由を集めて初期値
    pub fn number(&mut self, field: F) -> f64 {
        self.optional_number(field)
            .unwrap_or_else(|| panic!("{field:?} は空欄を受ける欄"))
    }

    /// 数値の欄。空欄は `None`。読めなければ理由を集めて初期値（初期値が空欄なら `None`）
    pub fn optional_number(&mut self, field: F) -> Option<f64> {
        match self.inputs.optional_number(field) {
            Ok(value) => value,
            Err(reason) => {
                self.refuse(field, reason);
                self.initial(field)
            }
        }
    }

    /// 空欄を受けない数値の欄を、計算核の型にする（`make`）。読めなければ文字の規則の理由、読めても型が
    /// 受けなければ `reason` を足し、どちらも初期値の型で埋める
    pub fn typed<T, E>(&mut self, field: F, make: impl Fn(f64) -> Result<T, E>, reason: &str) -> T {
        if let Ok(value) = make(self.number(field)) {
            return value;
        }
        self.refuse(field, reason);
        let initial = self.initial(field).expect("空欄を受けない欄の初期値は数");
        make(initial).unwrap_or_else(|_| panic!("{field:?} の初期値を型が受けない"))
    }

    /// 文字の欄。打ったままの文字
    pub fn text(&self, field: F) -> &'a str {
        assert!(
            matches!(field.definition().kind, Kind::Text { .. }),
            "{field:?} は文字の欄でない"
        );
        self.inputs.value(field)
    }

    /// チェックの欄
    pub fn checked(&self, field: F) -> bool {
        self.inputs.checked(field)
    }

    /// 選択の欄。選択肢の記号
    pub fn choice(&self, field: F) -> char {
        self.inputs.choice(field)
    }

    /// 計算核の入口が返した理由を足す。`field_of` が欄に写す理由はその欄の理由に、写らない理由（`None`）は
    /// `across` の欄に関わる欄をまたぐ理由になる。文言は理由の `Display`
    pub fn invalid<R: Copy + std::fmt::Display>(
        &mut self,
        reasons: &[R],
        field_of: impl Fn(R) -> Option<F>,
        across: &[F],
    ) {
        for &reason in reasons {
            match field_of(reason) {
                Some(field) => self.refuse(field, reason.to_string()),
                None => {
                    assert!(
                        !across.is_empty(),
                        "欄をまたぐ理由「{reason}」に関わる欄が無い"
                    );
                    self.across.push((across.to_vec(), reason.to_string()));
                }
            }
        }
    }

    /// 読むだけの欄に出す値を決める。決めない欄は「—」
    pub fn show(&mut self, field: F, value: impl Into<String>) {
        assert!(
            matches!(field.definition().kind, Kind::ReadOnly),
            "{field:?} は読むだけの欄でない"
        );
        self.shown.push((field, value.into()));
    }

    /// 警告を足す（計算はできるか、解けないことを言う）。`fields` は警告の関わる欄で、出す組を決める
    pub fn warn(&mut self, fields: &[F], warning: impl Into<String>) {
        assert!(!fields.is_empty(), "警告に関わる欄が無い");
        self.warnings.push((fields.to_vec(), warning.into()));
    }

    /// 理由のある欄か
    pub fn refused(&self, field: F) -> bool {
        self.reasons.iter().any(|(f, _)| *f == field)
    }

    /// 理由が 1 つでもあるか（欄ごとの理由か、欄をまたぐ理由）
    pub fn has_reasons(&self) -> bool {
        !self.reasons.is_empty() || !self.across.is_empty()
    }

    /// 入力の組の記録にする
    pub fn finish(mut self) -> InputRecord<F> {
        let index = Inputs::<F>::index;
        self.reasons.sort_by_key(|(field, _)| index(*field));
        let refused: Vec<F> = self.reasons.iter().map(|(f, _)| *f).collect();
        // 関わる欄のどれかに欄ごとの理由があれば、欄をまたぐ理由は言わない（クライテリアが読めないときに
        // 「1 つも無い」と言わない）
        self.across
            .retain(|(fields, _)| !fields.iter().any(|f| refused.contains(f)));
        for (fields, _) in self.across.iter_mut().chain(&mut self.warnings) {
            fields.sort_by_key(|field| index(*field));
        }
        InputRecord {
            reasons: self.reasons,
            across: self.across,
            warnings: self.warnings,
            shown: self.shown,
        }
    }

    /// 理由を足す
    fn refuse(&mut self, field: F, reason: impl Into<Reason>) {
        self.reasons.push((field, reason.into()));
    }

    /// 数値の欄の初期値
    fn initial(&self, field: F) -> Option<f64> {
        let Kind::Number { blank, initial } = field.definition().kind else {
            panic!("{field:?} は数値の欄でない");
        };
        read_number(initial, blank).unwrap_or_else(|_| panic!("{field:?} の初期値は読める"))
    }
}

/// 入力の組の記録（`GLOSSARY.md`「入力の組」）。欄ごとの理由（文字の規則、計算核の型が受けない値、計算核の
/// 入口の理由）、欄をまたぐ理由と関わる欄、警告と関わる欄、読むだけの欄の値。計算の結果が 1 つ持ち、入力の組にそのまま渡す。
/// 作れるのは [`Reading::finish`] だけ
#[derive(Debug, Clone, PartialEq)]
pub struct InputRecord<F> {
    /// 欄の並び（`Field::ALL`）の順
    reasons: Vec<(F, Reason)>,
    /// 関わる欄は欄の並びの順
    across: Vec<(Vec<F>, Reason)>,
    /// 関わる欄は欄の並びの順
    warnings: Vec<(Vec<F>, String)>,
    /// 読むだけの欄に出す値
    shown: Vec<(F, String)>,
}

impl<F: Copy + PartialEq> InputRecord<F> {
    /// 欄ごとの理由。欄の並び（`Field::ALL`）の順
    pub fn reasons(&self) -> &[(F, Reason)] {
        &self.reasons
    }

    /// 欄をまたぐ理由と、関わる欄（欄の並びの順）
    pub fn across(&self) -> &[(Vec<F>, Reason)] {
        &self.across
    }

    /// 警告と、関わる欄（欄の並びの順）
    pub fn warnings(&self) -> &[(Vec<F>, String)] {
        &self.warnings
    }

    /// 読むだけの欄に出す値。決めていなければ無し
    pub fn shown(&self, field: F) -> Option<&str> {
        self.shown
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, value)| value.as_str())
    }

    /// 理由が 1 つでもあるか。あるあいだ計算の結果の値は無い
    pub fn has_reasons(&self) -> bool {
        !self.reasons.is_empty() || !self.across.is_empty()
    }

    /// その欄に理由があるか（欄の枠が `ng`）
    pub fn refused(&self, field: F) -> bool {
        self.reasons.iter().any(|(f, _)| *f == field)
    }

    /// その欄に理由があるか、欄をまたぐ理由に関わるか（欄の見出しが `ng`）
    pub fn flagged(&self, field: F) -> bool {
        self.refused(field)
            || self
                .across
                .iter()
                .any(|(fields, _)| fields.contains(&field))
    }
}

/// Sheet の並べ方（`docs/ui-decisions.md`「入力と表示の区別」）
#[derive(Debug, Clone, PartialEq)]
pub enum Layout<F: Field> {
    /// 行見出し: 左に見出し、右に欄。行ごとに 1 欄か 2 組。2 組の行のある Sheet で 1 欄だけの行は、
    /// 空いた組を `fill` の空きで埋める
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
/// 理由、警告（「! …」）の順に `space-1` で積み、子（結果の表）をその下に `space-6` 空けて置く。
/// 切り替え（`switch`。Kirchhoff 板の材料）があれば、組のいちばん上に置いて Sheet との間を `space-2` 空ける。
/// 読むだけの欄の値は `record` が持つ（無ければ「—」）。
/// `record` はページの記録をそのまま受け、この Sheet の欄のものだけを出す。欄をまたぐ理由は、関わる欄の
/// うち最初の欄（`Field::ALL` の順）が載る組に 1 回だけ出し、関わる欄の見出しはそれぞれの組で `ng` にする。
/// 警告も関わる欄の最初の欄が載る組に出す（色は付けない）
#[component]
pub(crate) fn InputGroup<F: Field>(
    inputs: Signal<Inputs<F>>,
    layout: Layout<F>,
    #[props(default)] notes: Vec<String>,
    record: InputRecord<F>,
    switch: Option<Element>,
    children: Element,
) -> Element {
    let fields = layout.fields();
    let own: Vec<&(F, Reason)> = fields
        .iter()
        .flat_map(|field| record.reasons().iter().filter(move |(f, _)| f == field))
        .collect();
    // 関わる欄の最初の欄がこの Sheet に載るもの
    let here = |list: &'_ [(Vec<F>, String)]| -> Vec<String> {
        list.iter()
            .filter(|(involved, _)| involved.first().is_some_and(|f| fields.contains(f)))
            .map(|(_, text)| text.clone())
            .collect()
    };
    // 1 行の組の数の最大。これより少ない行は空きで埋める
    let pairs = match &layout {
        Layout::RowHeads(rows) => rows.iter().map(Vec::len).max().unwrap_or(0),
        Layout::ColumnHeads(_) => 0,
    };
    let across = here(record.across());
    let warnings = here(record.warnings());
    let has_error = |field: F| record.refused(field);
    let head_error = |field: F| record.flagged(field);
    let shown = |field: F| record.shown(field).unwrap_or(NO_VALUE).to_string();
    let stack = rsx! {
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
                                                Cell { inputs, field, error: has_error(field), shown: shown(field) }
                                            }
                                        }
                                        if row.len() < pairs {
                                            SheetBlank { span: 2 * (pairs - row.len()) }
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
                                            Cell { inputs, field, error: has_error(field), shown: shown(field) }
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
                for (field , reason) in own.iter() {
                    p { class: "text-label text-ng", "{field.definition().name}: {reason}" }
                }
                for reason in across.iter() {
                    p { class: "text-label text-ng", "{reason}" }
                }
                for warning in warnings.iter() {
                    p { class: "text-label", "! {warning}" }
                }
            }
    };
    rsx! {
        div { class: "flex flex-col gap-6",
            match switch {
                Some(switch) => rsx! {
                    div { class: "flex flex-col gap-2",
                        {switch}
                        {stack}
                    }
                },
                None => stack,
            }
            {children}
        }
    }
}

/// 1 つの欄。種類で見た目を選び、`aria-label` に欄の名前を付ける。`shown` は読むだけの欄に出す値
#[component]
fn Cell<F: Field>(inputs: Signal<Inputs<F>>, field: F, error: bool, shown: String) -> Element {
    let Definition { name, kind } = field.definition();
    match kind {
        Kind::ReadOnly => rsx! {
            SheetValue { label: name, value: shown }
        },
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
                onchange: move |checked: bool| inputs.write().set(field, checked.to_string()),
            }
        },
        Kind::Choice { options, .. } => rsx! {
            SheetSelect {
                label: name,
                options,
                selected: inputs.read().choice(field),
                onchange: move |symbol: char| inputs.write().set(field, symbol),
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::{count_tags, tags};

    /// 4 種類の欄を 1 つずつ持つ試験用の欄
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Test {
        /// 数値
        Positive,
        /// 数値（初期値に 3 桁区切り）
        Load,
        /// 数値
        Pressure,
        /// 空欄を受ける数値
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
                        initial: "4.0",
                    },
                ),
                Self::Load => (
                    "積載荷重（N/m²）",
                    Kind::Number {
                        blank: false,
                        initial: "1,800",
                    },
                ),
                Self::Pressure => (
                    "p（kN/m²）",
                    Kind::Number {
                        blank: false,
                        initial: "10",
                    },
                ),
                Self::Ratio => (
                    "変形角 1/",
                    Kind::Number {
                        blank: true,
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
        assert_eq!(inputs.number(Test::Positive), Err("値を入れる".into()));
        for text in ["abc", "inf", "NaN", "1e400", "1,2,3x"] {
            inputs.set(Test::Positive, text);
            assert_eq!(
                inputs.number(Test::Positive),
                Err("数値で入れる".into()),
                "{text}"
            );
        }
    }

    /// 0 や負を受けるかは計算核の入口が見る。欄は数値として読むだけ
    #[test]
    fn zero_and_negative_numbers_are_read_as_they_are() {
        let mut inputs = Inputs::<Test>::new();
        inputs.set(Test::Positive, "0");
        assert_eq!(inputs.number(Test::Positive), Ok(0.0));
        inputs.set(Test::Load, "-0.5");
        assert_eq!(inputs.number(Test::Load), Ok(-0.5));
        inputs.set(Test::Pressure, "-10");
        assert_eq!(inputs.number(Test::Pressure), Ok(-10.0));
    }

    #[test]
    fn a_field_that_accepts_a_blank_reads_the_blank_as_none() {
        let mut inputs = Inputs::<Test>::new();
        assert_eq!(inputs.optional_number(Test::Ratio), Ok(None));
        inputs.set(Test::Ratio, "400");
        assert_eq!(inputs.optional_number(Test::Ratio), Ok(Some(400.0)));
        inputs.set(Test::Ratio, "x");
        assert_eq!(
            inputs.optional_number(Test::Ratio),
            Err("数値で入れる".into())
        );
        // 空欄を受けない欄も同じ読み方で、空欄だけが理由になる
        assert_eq!(inputs.optional_number(Test::Load), Ok(Some(1800.0)));
        inputs.set(Test::Load, "");
        assert_eq!(inputs.optional_number(Test::Load), Err("値を入れる".into()));
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

    /// 計算核の入口が返す理由の試験用。`Name` は欄に写り、`OneOf` は写らない
    #[derive(Debug, Clone, Copy)]
    enum TestInvalid {
        Name,
        OneOf,
    }

    impl std::fmt::Display for TestInvalid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(match self {
                Self::Name => "この Sheet に無い欄",
                Self::OneOf => "どれか 1 つ",
            })
        }
    }

    fn field_of(reason: TestInvalid) -> Option<Test> {
        match reason {
            TestInvalid::Name => Some(Test::Name),
            TestInvalid::OneOf => None,
        }
    }

    /// 0 より大きい値だけを受ける計算核の型の代わり
    #[derive(Debug, PartialEq)]
    struct Positive(f64);

    fn positive(value: f64) -> Result<Positive, ()> {
        (value > 0.0).then_some(Positive(value)).ok_or(())
    }

    /// `InputGroup` を 1 つ置いたページ。読めない欄を欄の順でなく読み、Sheet に無い欄の理由も混ぜる。
    /// 選択の欄は初期値でない 2 つ目の選択肢を選んでおく
    #[component]
    fn GroupPage() -> Element {
        let inputs = use_signal(|| {
            let mut inputs = Inputs::<Test>::new();
            inputs.set(Test::Support, 'S');
            inputs.set(Test::Load, "");
            inputs.set(Test::Positive, "x");
            inputs
        });
        let record = {
            let inputs = inputs.read();
            let mut reading = inputs.reading();
            reading.number(Test::Load);
            reading.number(Test::Positive);
            reading.invalid(
                &[TestInvalid::Name, TestInvalid::OneOf],
                field_of,
                &[Test::Table, Test::Ratio],
            );
            reading.warn(&[Test::Positive], "想定の外");
            reading.finish()
        };
        rsx! {
            InputGroup {
                inputs,
                layout: Layout::RowHeads(vec![
                    vec![Test::Positive, Test::Load],
                    vec![Test::Ratio],
                    vec![Test::Table, Test::Support],
                ]),
                notes: vec!["注記".into()],
                record,
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
            "単純支持",
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
        // エラーの見出しは、読めない 2 欄とまたぐエラーの 2 欄（data-state）。不正な欄は読めない 2 欄だけ
        // （aria-invalid。またぐエラーの欄は読めている）
        assert_eq!(
            count_tags(&html, "th", &[("data-state", "error")]),
            4,
            "{html}"
        );
        assert_eq!(
            count_tags(&html, "input", &[("aria-invalid", "true")]),
            2,
            "{html}"
        );
        assert_eq!(
            count_tags(&html, "input", &[("aria-label", "Lx（m）")]),
            1,
            "{html}"
        );
        // チェックはボタンの checkbox で、初期値のオフを aria-checked に持つ
        assert_eq!(
            count_tags(
                &html,
                "button",
                &[
                    ("role", "checkbox"),
                    ("aria-checked", "false"),
                    ("aria-label", "表18-1 最小厚さ"),
                ]
            ),
            1,
            "{html}"
        );
        // 選択はリストを開くボタンで、閉じている間は選んだ選択肢の名前だけを出す
        assert_eq!(
            count_tags(
                &html,
                "button",
                &[
                    ("aria-haspopup", "listbox"),
                    ("aria-expanded", "false"),
                    ("aria-label", "x = 0（左）"),
                ]
            ),
            1,
            "{html}"
        );
        assert!(!html.contains("固定"), "{html}");
    }

    /// 選択の欄のリストは位置を指定しない `absolute` で、包む箱が Sheet の外になるので、Sheet の
    /// `overflow-hidden` に切られない。Sheet の中のどの要素も位置を決めるクラスを持たない
    #[test]
    fn nothing_in_the_sheet_is_positioned_so_the_choice_list_escapes_its_overflow() {
        let mut dom = VirtualDom::new(GroupPage);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let positioned: Vec<_> = tags(&html)
            .iter()
            .flat_map(|tag| tag.classes())
            .filter(|class| ["relative", "absolute", "fixed", "sticky"].contains(class))
            .map(str::to_string)
            .collect();
        assert_eq!(positioned, Vec::<String>::new(), "{html}");
    }

    /// 読めない欄は理由を集めて初期値で埋め（空欄を受ける欄の空欄の初期値は `None`）、理由は欄の順に並ぶ
    #[test]
    fn a_reading_fills_unreadable_fields_with_their_initial_values_in_field_order() {
        let mut inputs = Inputs::<Test>::new();
        inputs.set(Test::Positive, "x");
        inputs.set(Test::Ratio, "1/400");
        inputs.set(Test::Pressure, "-10");
        let mut reading = inputs.reading();
        let ratio = reading.optional_number(Test::Ratio);
        let pressure = reading.number(Test::Pressure);
        let positive = reading.number(Test::Positive);
        assert!(!reading.refused(Test::Load));
        assert_eq!((ratio, pressure, positive), (None, -10.0, 4.0));
        let record = reading.finish();
        assert_eq!(
            record.reasons(),
            [
                (Test::Positive, "数値で入れる".to_string()),
                (Test::Ratio, "数値で入れる".to_string()),
            ]
        );
        assert!(record.has_reasons());
    }

    /// 読めても型が受けない値は、渡した文言の理由を足して初期値の型で埋める。読めない値は文字の規則の理由で、
    /// やはり初期値の型。受ける値はそのまま
    #[test]
    fn a_value_the_type_refuses_is_named_and_filled_with_the_initial_value_of_the_type() {
        let mut inputs = Inputs::<Test>::new();
        inputs.set(Test::Pressure, "-10");
        inputs.set(Test::Positive, "x");
        inputs.set(Test::Load, "2,000");
        let mut reading = inputs.reading();
        let reason = "0 より大きい値を入れる";
        assert_eq!(
            reading.typed(Test::Pressure, positive, reason),
            Positive(10.0)
        );
        assert_eq!(
            reading.typed(Test::Positive, positive, reason),
            Positive(4.0)
        );
        assert_eq!(
            reading.typed(Test::Load, positive, reason),
            Positive(2000.0)
        );
        let record = reading.finish();
        assert_eq!(
            record.reasons(),
            [
                (Test::Positive, "数値で入れる".to_string()),
                (Test::Pressure, reason.to_string()),
            ]
        );
    }

    /// 計算核の理由は、写る欄の理由と、写らないものは関わる欄（欄の並びの順）と一緒に欄をまたぐ理由になる
    #[test]
    fn the_reasons_of_the_entry_go_to_their_fields_or_across_the_given_fields() {
        let inputs = Inputs::<Test>::new();
        let mut reading = inputs.reading();
        reading.invalid(
            &[TestInvalid::OneOf, TestInvalid::Name],
            field_of,
            &[Test::Table, Test::Ratio],
        );
        assert!(reading.refused(Test::Name) && reading.has_reasons());
        let record = reading.finish();
        assert_eq!(
            record.reasons(),
            [(Test::Name, "この Sheet に無い欄".to_string())]
        );
        assert_eq!(
            record.across(),
            [(vec![Test::Ratio, Test::Table], "どれか 1 つ".to_string())]
        );
        assert!(record.flagged(Test::Ratio) && !record.refused(Test::Ratio));
        assert!(record.flagged(Test::Name) && !record.flagged(Test::Load));
    }

    /// 関わる欄のどれかに理由があれば、欄をまたぐ理由は言わない（読めないクライテリアは「無い」と言わない）
    #[test]
    fn a_reason_across_fields_is_dropped_when_one_of_them_has_its_own_reason() {
        let mut inputs = Inputs::<Test>::new();
        inputs.set(Test::Ratio, "x");
        let mut reading = inputs.reading();
        reading.optional_number(Test::Ratio);
        reading.invalid(&[TestInvalid::OneOf], field_of, &[Test::Ratio, Test::Table]);
        let record = reading.finish();
        assert_eq!(record.across(), []);
        assert_eq!(record.reasons().len(), 1);
    }

    /// 2 つの Sheet を持つページ。欄をまたぐ理由は Load（上の組）と Table（下の組）に関わり、警告は Support と
    /// Ratio（どちらも下の組）に関わる
    #[component]
    fn TwoGroupsPage() -> Element {
        let inputs = use_signal(Inputs::<Test>::new);
        let record = {
            let inputs = inputs.read();
            let mut reading = inputs.reading();
            reading.invalid(&[TestInvalid::OneOf], field_of, &[Test::Table, Test::Load]);
            reading.warn(&[Test::Support, Test::Ratio], "想定の外");
            reading.finish()
        };
        rsx! {
            InputGroup {
                inputs,
                layout: Layout::RowHeads(vec![vec![Test::Positive], vec![Test::Load]]),
                record: record.clone(),
                p { "上の組の結果" }
            }
            InputGroup {
                inputs,
                layout: Layout::ColumnHeads(vec![Test::Ratio, Test::Table, Test::Support]),
                record,
                p { "下の組の結果" }
            }
        }
    }

    /// 欄をまたぐ理由は、関わる欄のうち最初の欄（欄の並びの順）が載る組に 1 回だけ出る。関わる欄の見出しは
    /// どちらの組でも `ng`。警告も関わる欄の最初の欄が載る組に出る
    #[test]
    fn a_reason_across_fields_is_said_once_under_the_group_of_its_first_field() {
        let mut dom = VirtualDom::new(TwoGroupsPage);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let (upper, lower) = html.split_at(html.find("上の組の結果").expect("上の組"));
        assert_eq!(upper.matches("どれか 1 つ").count(), 1, "{html}");
        assert_eq!(lower.matches("どれか 1 つ").count(), 0, "{html}");
        assert_eq!(upper.matches("想定の外").count(), 0, "{html}");
        assert_eq!(lower.matches("! 想定の外").count(), 1, "{html}");
        let error = [("data-state", "error")];
        assert_eq!(count_tags(upper, "th", &error), 1, "{html}");
        assert_eq!(count_tags(lower, "th", &error), 1, "{html}");
        // 欄をまたぐ理由の欄は読めているので、欄の枠は `ng` でない
        assert_eq!(
            count_tags(&html, "input", &[("aria-invalid", "true")]),
            0,
            "{html}"
        );
    }
}
