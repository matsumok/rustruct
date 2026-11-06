//! 計算のページ（Data と Tool）の表示用の表と本文の列の部品。
//! 決め事は ADR site-0006 と `docs/ui-decisions.md`「列の単位と表」で、部品がそれを守るので使う側は書かない:
//!
//! - 表は横罫だけ。見出しの行と本体の各行の下に `line`。縦罫・塗り・交互の背景は使わない（入力の Sheet と分ける）
//! - 単位は表の名前（`Caption`）か列の見出し（`TableColumn` の `unit`）に括弧で書き、数字の後には付けない
//! - 入力で値が変わる表は `Table` の `fixed` で列の幅を固定する
//! - 表は列の定義（`TableColumn`）と行の値（`Cell`）で宣言し、揃え・小数点揃え・見出しの改行・横スクロール・
//!   「—」は `Table` が決める。数の書き方（有効数字、3 桁区切り、E）は `numerus` で、値を作る側が決める
//! - 本文は1単位（`width-column`）の列（`Column`）を `Columns` で並べ、「入力＋結果」と根拠の間に `Divider`
//! - 表や式は `Block` に、見出し（`Caption`）、表や式、注記（`Notes`）の順で置く。出典は画面に出さない

use dioxus::prelude::*;
use numerus::{FIGURE_SPACE, point_pads};

/// 本文の列を並べる（ui-decisions「列の単位」）。iPad・PC では1単位（`width-column`）の列と区切りを横に並べ、
/// 間は tokens の `layout` の `gap`（iPad `space-4`、PC `space-9`。区切りの線の左右が同じ空きになる）。スマホでは縦に積み、間は `space-8`
/// （`space-7` は表の行の高さとほぼ同じで、区切りが表の次の行の罫に見えたため）。空きはすべてここで決め、
/// 列と区切りは自分の外に余白を持たない。本文の箱の幅（`max-w-page` / `lg:max-w-page-lg`）は、
/// 同じ `gap` と1単位から `theme.rs` が導く。
#[component]
pub(crate) fn Columns(children: Element) -> Element {
    rsx! {
        div { class: "flex flex-col gap-8 md:flex-row md:items-start md:gap-gap-md lg:gap-gap-lg", {children} }
    }
}

/// 本文の1列。スマホでは画面いっぱい、iPad からは1単位の幅（ADR site-0007）。中の組（`Block` など）の間は `space-7`。
/// `id` を付けると住所の `#id` でその列へ飛べる。
#[component]
pub(crate) fn Column(#[props(default)] id: &'static str, children: Element) -> Element {
    rsx! {
        div {
            class: "flex w-full min-w-0 flex-col gap-7 md:max-w-column",
            id: (!id.is_empty()).then_some(id),
            {children}
        }
    }
}

/// 列と列の間の `line` の区切り。入力と結果の列と、入力に依らない根拠の列を分ける。
/// スマホでは横の線、iPad・PC では高さいっぱいの縦の線（Tailwind の初期化が `hr` に高さ 0 を当てるので `h-auto` で戻す）。
/// 前後の空きは `Columns` が持つ。
#[component]
pub(crate) fn Divider() -> Element {
    rsx! {
        hr { class: "border-line md:h-auto md:self-stretch md:border-t-0 md:border-l" }
    }
}

/// 表や式の一まとまり。見出し（`Caption`）、表や式、注記の順で、間は `space-2`。
/// 部の見出し（入力・結果・根拠）は置かず、この見出しで区切る（ui-decisions「列の単位と表」）。
#[component]
pub(crate) fn Block(children: Element) -> Element {
    rsx! {
        section { class: "flex flex-col gap-2", {children} }
    }
}

/// 表や式の見出し。`heading`（太字・ink）。
#[component]
pub(crate) fn Caption(children: Element) -> Element {
    rsx! {
        h2 { class: "text-heading", {children} }
    }
}

/// 表や式に添える注記の箇条。
#[component]
pub(crate) fn Notes(children: Element) -> Element {
    rsx! {
        ul { class: "list-disc pl-4 text-label text-muted", {children} }
    }
}

/// 表の値が無いときの字（`docs/ui-decisions.md`「エラーと警告」）。計算できないときと、値はあるが
/// その位置や上限が無いとき（辺の全体で 0 の値の位置、たわみに効く条件が無い許容たわみ）の両方に使う
pub(crate) const NO_VALUE: &str = "—";

/// 表の列の定義。種類（名前・数値・括弧書き・式）、見出し、単位、まとまり
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TableColumn {
    kind: Kind,
    head: &'static str,
    unit: Option<&'static str>,
    group: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Name,
    Number,
    Paren,
    Formula,
}

/// 表のセルの値。値か無し（`None`、「—」で描く）か、式
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Cell {
    Value(Option<String>),
    Formula(Formula),
}

/// 1単位に収まる列の数。鉄筋の呼び名の表（名前と値 4 つ）が収まり、これを超える表は横にスクロールする
const COLUMNS_IN_UNIT: usize = 5;

/// 入力で値が変わる表（`fixed`）の値の列の上限（ADR site-0006）
const FIXED_NUMBER_COLUMNS: usize = 3;

/// 表示用の表（ui-decisions「列の単位と表」）。列の定義（`TableColumn`）と行の値（`Cell`）を受け、形はここが決める:
///
/// - 横罫だけで、線はすべて `line`（`ink` は部の見出しの上だけ）。縦罫と塗りは使わない。幅は列いっぱい（1単位）
/// - セルは左に `space-3` を取り、最初の列だけ左を詰めて右に `space-3`（表の端の列を端に寄せる）。
///   位置で変わる形は CSS の `first:` で決め、セルに「最初かどうか」を渡さない
/// - 揃えは列の種類で決める。名前と式の列は左、数値の列は右で、小数の桁数が行で違えば足りない桁を
///   数字幅の空白（U+2007）で埋めて小数点を揃える。値の無いセルは「—」（`NO_VALUE`）
/// - 見出しと名前の `\n` は `br` で改行する（列見出しは勝手には折り返さない）。単位は見出しの後に括弧で
/// - まとまり（`group`）のある列は、まとまりの見出しの行の下に並べる。括弧書きの列（`paren`）は前の数値の列と
///   見出しとまとまりを分け合う
/// - 列が1単位に入りきらない表（`COLUMNS_IN_UNIT` を超える）は、その表だけ横にスクロールする
/// - `fixed` の表は列の幅を中身で変えず、すべての列を等分する（入力で値が変わっても列が動かない）
#[component]
pub(crate) fn Table(
    columns: Vec<TableColumn>,
    rows: Vec<Vec<Cell>>,
    #[props(default)] fixed: bool,
) -> Element {
    debug_assert!(
        rows.iter().all(|row| row.len() == columns.len()),
        "行のセルの数が列の数と違う"
    );
    if fixed {
        debug_assert!(
            columns.iter().filter(|c| c.kind == Kind::Number).count() <= FIXED_NUMBER_COLUMNS,
            "入力で変わる表の値の列は 3 つまで"
        );
    }
    let pads: Vec<Vec<usize>> = (0..columns.len())
        .map(|i| match columns[i].kind {
            Kind::Number => point_pads(rows.iter().map(|row| row[i].text())),
            _ => vec![0; rows.len()],
        })
        .collect();
    let heads = heads(&columns);
    let table = rsx! {
        table { class: if fixed { "w-full table-fixed" } else { "w-full" },
            thead {
                if let Some(groups) = heads.groups {
                    tr {
                        for head in groups {
                            {head.render()}
                        }
                    }
                }
                tr { class: "border-b border-line",
                    for head in heads.columns {
                        {head.render()}
                    }
                }
            }
            tbody {
                for (r , row) in rows.iter().enumerate() {
                    tr { class: "border-b border-line",
                        for (c , cell) in row.iter().enumerate() {
                            {body_cell(columns[c].kind, cell, pads[c][r])}
                        }
                    }
                }
            }
        }
    };
    if columns.len() > COLUMNS_IN_UNIT {
        rsx! {
            div { class: "overflow-x-auto", {table} }
        }
    } else {
        table
    }
}

/// 見出しの行。まとまりのある表は、まとまりの見出しの行（`groups`）と列見出しの行の 2 行
struct Heads {
    groups: Option<Vec<Head>>,
    columns: Vec<Head>,
}

/// 見出しのセル
enum Head {
    /// 列見出し。`left` は名前と式の列。`span` は括弧書きの列とのまたぎ、`rows` はまとまりの見出しの行とのまたぎ
    Column {
        left: bool,
        text: String,
        span: Option<u32>,
        rows: Option<u32>,
    },
    /// まとまりの見出し。`span` 列にまたがる
    Group { text: &'static str, span: u32 },
}

/// 列の定義から見出しの行を組む。括弧書きの列は前の列の見出しとまとまりに入る
fn heads(columns: &[TableColumn]) -> Heads {
    let mut groups: Vec<Head> = Vec::new();
    let mut heads: Vec<Head> = Vec::new();
    let has_groups = columns.iter().any(|c| c.group.is_some());
    let mut group: Option<&'static str> = None;
    for column in columns {
        if column.kind == Kind::Paren {
            if let Some(Head::Column { span, .. }) = heads.last_mut() {
                *span = Some(span.unwrap_or(1) + 1);
            }
            if group.is_some()
                && let Some(Head::Group { span, .. }) = groups.last_mut()
            {
                *span += 1;
            }
            continue;
        }
        let head = |rows| Head::Column {
            left: column.kind != Kind::Number,
            text: column.head_text(),
            span: None,
            rows,
        };
        group = column.group;
        match column.group {
            None if has_groups => groups.push(head(Some(2))),
            None => heads.push(head(None)),
            Some(text) => {
                match groups.last_mut() {
                    Some(Head::Group { text: last, span }) if *last == text => *span += 1,
                    _ => groups.push(Head::Group { text, span: 1 }),
                }
                heads.push(head(None));
            }
        }
    }
    Heads {
        groups: has_groups.then_some(groups),
        columns: heads,
    }
}

impl Head {
    /// 列見出しは `label` の muted で、上下は中央（2行の見出しと並ぶ1行の見出しも）、折り返さない。
    /// 数値の列の見出しは、改行した行どうしを左で揃え、そのまとまりを列の右に寄せる。
    /// まとまりの見出しは中央に置いて下に `line`。線はセルの左の空きを除いた中身に引き、隣のまとまりの線と
    /// `space-3` 離す
    fn render(&self) -> Element {
        match self {
            Self::Column {
                left: true,
                text,
                span,
                rows,
            } => rsx! {
                th {
                    colspan: *span,
                    rowspan: *rows,
                    class: "py-2 pl-3 first:pl-0 first:pr-3 text-left text-label text-muted font-normal whitespace-nowrap",
                    {lines(text)}
                }
            },
            Self::Column {
                left: false,
                text,
                span,
                rows,
            } => rsx! {
                th {
                    colspan: *span,
                    rowspan: *rows,
                    class: "py-2 pl-3 first:pl-0 first:pr-3 text-right text-label text-muted font-normal whitespace-nowrap",
                    span { class: "inline-block text-left", {lines(text)} }
                }
            },
            Self::Group { text, span } => rsx! {
                th {
                    colspan: *span,
                    class: "pt-2 pl-3 first:pl-0 first:pr-3 text-center text-label text-muted font-normal whitespace-nowrap",
                    div { class: "pb-2 border-b border-line", {*text} }
                }
            },
        }
    }
}

/// 本体のセル。名前は `body` で左、数値は `num` の等幅で右（足りない桁を `pad` 個の U+2007 で右に埋め、
/// 埋めた空白は読み上げから外す）。括弧書きは左を詰めて左に揃え、括弧の外の数値が括弧の無い行の数値と
/// 右端で揃う。式は `num` で左、勝手には折り返さない
fn body_cell(kind: Kind, cell: &Cell, pad: usize) -> Element {
    match (kind, cell) {
        (_, Cell::Formula(formula)) => rsx! {
            td { class: "py-2 pl-3 first:pl-0 first:pr-3 font-mono text-num whitespace-nowrap",
                match *formula {
                    Formula::Plain(f) => rsx! { {f} },
                    Formula::Min(a, b) => rsx! {
                        "min("
                        span { class: "inline-block align-top", "{a}," br {} "{b})" }
                    },
                }
            }
        },
        (Kind::Name, cell) => rsx! {
            td { class: "py-2 pl-3 first:pl-0 first:pr-3 text-body", {lines(cell.text())} }
        },
        (Kind::Formula, cell) => rsx! {
            td { class: "py-2 pl-3 first:pl-0 first:pr-3 font-mono text-num whitespace-nowrap", {cell.text()} }
        },
        (Kind::Number, cell) => {
            let pad = FIGURE_SPACE.to_string().repeat(pad);
            rsx! {
                td { class: "py-2 pl-3 first:pl-0 first:pr-3 font-mono text-num text-right whitespace-nowrap",
                    {cell.text()}
                    if !pad.is_empty() {
                        span { aria_hidden: "true", {pad} }
                    }
                }
            }
        }
        (Kind::Paren, cell) => rsx! {
            td { class: "py-2 font-mono text-num whitespace-nowrap", {cell.text()} }
        },
    }
}

/// `\n` で分けた行を `br` でつなぐ
fn lines(text: &str) -> Element {
    rsx! {
        for (i , line) in text.split('\n').enumerate() {
            if i > 0 {
                br {}
            }
            {line}
        }
    }
}

/// 式のセルの式。`Min` は min(A, B) で、A の後で折り返して B を A の下に揃える。
/// 1行の min(A, B) では式の列が広く、名前の列が語の途中で切れたため。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Formula {
    Plain(&'static str),
    Min(&'static str, &'static str),
}

impl TableColumn {
    pub(crate) const fn name(head: &'static str) -> Self {
        Self::of(Kind::Name, head)
    }
    pub(crate) const fn number(head: &'static str) -> Self {
        Self::of(Kind::Number, head)
    }
    pub(crate) const fn formula(head: &'static str) -> Self {
        Self::of(Kind::Formula, head)
    }
    pub(crate) const fn paren() -> Self {
        Self::of(Kind::Paren, "")
    }
    pub(crate) const fn unit(self, unit: &'static str) -> Self {
        Self {
            unit: Some(unit),
            ..self
        }
    }
    pub(crate) const fn group(self, group: &'static str) -> Self {
        Self {
            group: Some(group),
            ..self
        }
    }
    const fn of(kind: Kind, head: &'static str) -> Self {
        Self {
            kind,
            head,
            unit: None,
            group: None,
        }
    }
}

impl TableColumn {
    /// 見出しの文字。単位は最後の行の後に括弧で
    fn head_text(&self) -> String {
        match self.unit {
            Some(unit) => format!("{}（{unit}）", self.head),
            None => self.head.to_string(),
        }
    }
}

impl Cell {
    /// 描く文字。値の無いセルは「—」。式は描く形が別なので `Table` が見ない
    fn text(&self) -> &str {
        match self {
            Self::Value(Some(value)) => value,
            Self::Value(None) => NO_VALUE,
            Self::Formula(_) => "",
        }
    }
}

impl From<Option<String>> for Cell {
    fn from(value: Option<String>) -> Self {
        Self::Value(value)
    }
}

impl From<String> for Cell {
    fn from(value: String) -> Self {
        Self::Value(Some(value))
    }
}

impl From<&str> for Cell {
    fn from(value: &str) -> Self {
        Self::Value(Some(value.into()))
    }
}

impl From<Formula> for Cell {
    fn from(formula: Formula) -> Self {
        Self::Formula(formula)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(columns: Vec<TableColumn>, rows: Vec<Vec<Cell>>, fixed: bool) -> String {
        let mut dom = VirtualDom::new_with_props(
            Table,
            TableProps {
                columns,
                rows,
                fixed,
            },
        );
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    /// タグの名前の並び（属性は見ない）。要素の分かれ方を見る
    fn tags(html: &str) -> Vec<&str> {
        html.split('<')
            .skip(1)
            .map(|tag| tag.split([' ', '>']).next().unwrap_or_default())
            .collect()
    }

    fn text_of(html: &str) -> String {
        let mut out = String::new();
        let mut in_tag = false;
        for c in html.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => out.push(c),
                _ => {}
            }
        }
        out
    }

    fn some(v: &str) -> Cell {
        Some(v.to_string()).into()
    }

    #[test]
    fn heads_go_in_thead_and_values_in_tbody_with_one_cell_per_column() {
        let html = render(
            vec![
                TableColumn::name("辺"),
                TableColumn::number("M").unit("kN·m/m"),
            ],
            vec![
                vec!["x = 0".into(), some("1.25")],
                vec!["y = 0".into(), some("12.7")],
            ],
            false,
        );
        assert_eq!(
            tags(&html),
            [
                "table", "thead", "tr", "th", "/th", "th", "span", "/span", "/th", "/tr", "/thead",
                "tbody", "tr", "td", "/td", "td", "/td", "/tr", "tr", "td", "/td", "td", "span",
                "/span", "/td", "/tr", "/tbody", "/table"
            ],
            "{html}"
        );
        assert_eq!(
            text_of(&html),
            format!("辺M（kN·m/m）x = 01.25y = 012.7{FIGURE_SPACE}")
        );
    }

    #[test]
    fn each_number_column_pads_its_shorter_decimals_with_figure_spaces() {
        let html = render(
            vec![TableColumn::number("a"), TableColumn::number("b")],
            vec![
                vec![some("12.7"), some("1,140")],
                vec![some("1.25"), some("8.95")],
                vec![None.into(), some("100")],
            ],
            true,
        );
        let pads: Vec<usize> = html
            .split("</td>")
            .map(|cell| cell.matches(FIGURE_SPACE).count())
            .collect();
        // 「—」は小数部が無いので、列の最も長い小数部（2 桁と小数点）の分を埋める
        assert_eq!(pads, [1, 3, 0, 0, 3, 3, 0], "{html}");
        assert!(text_of(&html).contains(NO_VALUE), "{html}");
    }

    #[test]
    fn a_line_break_in_a_head_or_a_name_becomes_a_br() {
        let html = render(
            vec![
                TableColumn::name(""),
                TableColumn::number("公称直径\nd").unit("mm"),
            ],
            vec![vec!["付着\n（上端筋）".into(), some("4.23")]],
            false,
        );
        for broken in ["公称直径<br/>d（mm）", "付着<br/>（上端筋）"] {
            assert!(html.contains(broken), "{broken} が無い: {html}");
        }
    }

    #[test]
    fn a_formula_column_stacks_the_two_arguments_of_min() {
        let html = render(
            vec![TableColumn::name(""), TableColumn::formula("長期")],
            vec![
                vec!["圧縮".into(), Formula::Plain("Fc/3").into()],
                vec!["せん断".into(), Formula::Min("Fc/30", "0.49+Fc/100").into()],
            ],
            false,
        );
        assert!(html.contains("Fc/30,<br/>0.49+Fc/100)"), "{html}");
        assert_eq!(text_of(&html), "長期圧縮Fc/3せん断min(Fc/30,0.49+Fc/100)");
    }

    #[test]
    fn groups_head_their_columns_and_a_paren_shares_the_head_of_its_number() {
        let html = render(
            vec![
                TableColumn::name("種類"),
                TableColumn::number("引張・圧縮").group("長期"),
                TableColumn::paren(),
                TableColumn::number("せん断").group("長期"),
                TableColumn::number("引張・圧縮").group("短期"),
                TableColumn::number("せん断").group("短期"),
            ],
            vec![vec![
                "SD345".into(),
                some("215"),
                "（*195）".into(),
                some("195"),
                some("345"),
                some("345"),
            ]],
            false,
        );
        let head = &html[..html.find("<tbody").expect("tbody がある")];
        assert_eq!(head.matches("<tr").count(), 2, "{head}");
        assert_in_order(
            head,
            &[
                "rowspan=2",
                "種類",
                "colspan=3",
                "長期",
                "colspan=2",
                "短期",
                "colspan=2",
                "引張・圧縮",
                "せん断",
                "引張・圧縮",
                "せん断",
            ],
        );
        // 値の列が 1 単位に入りきらないので、表だけ横にスクロールする箱に入れる
        assert!(html.starts_with("<div"), "{html}");
        assert_eq!(
            text_of(&html[html.find("<tbody").unwrap()..]),
            "SD345215（*195）195345345"
        );
    }

    #[test]
    fn a_table_that_fits_the_column_is_not_wrapped() {
        let html = render(
            vec![
                TableColumn::name("呼び名"),
                TableColumn::number("d"),
                TableColumn::number("l"),
                TableColumn::number("S"),
                TableColumn::number("w"),
            ],
            vec![vec![
                "D4".into(),
                some("4.23"),
                some("13.3"),
                some("14.05"),
                some("0.110"),
            ]],
            false,
        );
        assert!(html.starts_with("<table"), "{html}");
    }

    fn assert_in_order(haystack: &str, needles: &[&str]) {
        let mut from = 0;
        for n in needles {
            let at = haystack[from..]
                .find(n)
                .unwrap_or_else(|| panic!("「{n}」が無い（またはこの順でない）: {haystack}"));
            from += at + n.len();
        }
    }
}
