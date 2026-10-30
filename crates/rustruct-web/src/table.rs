//! 計算のページ（Data と Tool）の表示用の表と本文の列の部品。
//! 決め事は `docs/adr/0006` と `docs/ui-decisions.md`「列の単位と表」で、部品がそれを守るので使う側は書かない:
//!
//! - 表は横罫だけ。見出しの行と本体の各行の下に `line`。縦罫・塗り・交互の背景は使わない（入力の Sheet と分ける）
//! - 単位は表の名前（`Caption`）か列の見出し（`HeadCell`）に括弧で書き、数字の後には付けない
//! - 入力で値が変わる表は `Table` の `fixed` で列の幅を固定する
//! - 小数の桁数が行で違う列は `DecimalCell` で小数点を揃える。数の書き方（有効数字、3 桁区切り、E）は `numerus`
//! - 本文は1単位（`width-column`）の列（`Column`）を `Columns` で並べ、「入力＋結果」と根拠の間に `Divider`
//! - 表や式は `Block` に、見出し（`Caption`）、表や式、注記（`Notes`）の順で置く。出典は画面に出さない

use dioxus::prelude::*;
use numerus::{FIGURE_SPACE, point_pads};

/// 本文の列を並べる（ui-decisions「列の単位」）。iPad・PC では1単位（`width-column`）の列と区切りを横に並べ、
/// 間は iPad `space-4`、PC `space-9`（区切りの線の左右が同じ空きになる）。スマホでは縦に積み、間は `space-8`
/// （`space-7` は表の行の高さとほぼ同じで、区切りが表の次の行の罫に見えたため）。空きはすべてここで決め、
/// 列と区切りは自分の外に余白を持たない。本文の箱の幅（`max-w-page` / `lg:max-w-page-lg`）は、
/// この間と1単位から `theme.rs` が導く。
#[component]
pub(crate) fn Columns(children: Element) -> Element {
    rsx! {
        div { class: "flex flex-col gap-8 md:flex-row md:items-start md:gap-4 lg:gap-9", {children} }
    }
}

/// 本文の1列。スマホでは画面いっぱい、iPad からは1単位の幅（`docs/adr/0007`）。中の組（`Block` など）の間は `space-7`。
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

/// 表示用の表（ui-decisions「列の単位と表」）。横罫だけで、線はすべて `line`（`ink` は部の見出しの上だけ）。
/// 縦罫と塗りは使わない。幅は列いっぱい（1単位）。入りきらない表は外側を `overflow-x-auto` で包む。
/// セルは左に `space-3` を取り、最初の列だけ左を詰めて右に `space-3`（表の端の列を端に寄せる）。
/// 位置で変わる形は CSS の `first:` で決め、セルに「最初かどうか」を渡さない。
/// `fixed` の表は列の幅を中身で変えず、すべての列を等分する（入力で値が変わっても列が動かない）。
#[component]
pub(crate) fn Table(#[props(default)] fixed: bool, children: Element) -> Element {
    rsx! {
        table { class: if fixed { "w-full table-fixed" } else { "w-full" }, {children} }
    }
}

/// 表の見出しの行。下に `line`。
#[component]
pub(crate) fn HeadRow(children: Element) -> Element {
    rsx! {
        tr { class: "border-b border-line", {children} }
    }
}

/// 表の本体の行。下に `line`（最後の行も）。
#[component]
pub(crate) fn Row(children: Element) -> Element {
    rsx! {
        tr { class: "border-b border-line", {children} }
    }
}

/// 列のまとまりの見出しの行（「長期」「短期」）。罫はまとまりの見出し（`GroupCell`）の下だけに引く。
#[component]
pub(crate) fn GroupRow(children: Element) -> Element {
    rsx! {
        tr { {children} }
    }
}

/// 列のまとまりの見出し。`span` 列にまたがり、中央に置いて下に `line`。線はセルの左の空きを除いた
/// 中身に引き、隣のまとまりの線と `space-3` 離す。
#[component]
pub(crate) fn GroupCell(span: u32, children: Element) -> Element {
    rsx! {
        th {
            colspan: span,
            class: "pt-2 pl-3 first:pl-0 first:pr-3 text-center text-label text-muted font-normal whitespace-nowrap",
            div { class: "pb-2 border-b border-line", {children} }
        }
    }
}

/// 列見出し。`label` の muted で、上下は中央（2行の見出しと並ぶ1行の見出しも）、折り返さない（改行は `br` で決める）。数値の列の見出しは、
/// 改行した行どうしを左で揃え、そのまとまりを列の右に寄せる。揃えは列の中身に合わせる: `left` の列（名前・区分の
/// `TextCell`、式の `FormulaCell`）は左、数値の列（`NumCell`）は右。`span` は列のまたぎ、`rows` は
/// 行のまたぎ（まとまりの見出しの行と並ぶ先頭の列）。
#[component]
pub(crate) fn HeadCell(
    #[props(default)] left: bool,
    span: Option<u32>,
    rows: Option<u32>,
    children: Element,
) -> Element {
    rsx! {
        th {
            colspan: span,
            rowspan: rows,
            class: if left { "py-2 pl-3 first:pl-0 first:pr-3 text-left text-label text-muted font-normal whitespace-nowrap" } else { "py-2 pl-3 first:pl-0 first:pr-3 text-right text-label text-muted font-normal whitespace-nowrap" },
            if left {
                {children}
            } else {
                span { class: "inline-block text-left", {children} }
            }
        }
    }
}

/// 名前や区分のセル。`body` で左揃え。
#[component]
pub(crate) fn TextCell(children: Element) -> Element {
    rsx! {
        td { class: "py-2 pl-3 first:pl-0 first:pr-3 text-body", {children} }
    }
}

/// 数値のセル。`num` の等幅で右揃え。
#[component]
pub(crate) fn NumCell(children: Element) -> Element {
    rsx! {
        td { class: "py-2 pl-3 first:pl-0 first:pr-3 font-mono text-num text-right whitespace-nowrap", {children} }
    }
}

/// 小数点の位置を列で揃える数値のセル。`pad` はその列の値の並びから `numerus::point_pads` で求めた埋めの数。
/// 足りない桁（と小数点）を数字幅の空白（U+2007）で右に埋める。等幅なので小数点が縦に揃う。埋めた空白は読み上げから外す。
#[component]
pub(crate) fn DecimalCell(value: String, pad: usize) -> Element {
    let pad = FIGURE_SPACE.to_string().repeat(pad);
    rsx! {
        td { class: "py-2 pl-3 first:pl-0 first:pr-3 font-mono text-num text-right whitespace-nowrap",
            {value}
            if !pad.is_empty() {
                span { aria_hidden: "true", {pad} }
            }
        }
    }
}

/// 数値に添える括弧書きのセル（「215（*195）」の「（*195）」）。前の `NumCell` と対で、左を詰めて
/// 左に揃える。括弧の外の数値が、括弧の無い行の数値と右端で揃う。
#[component]
pub(crate) fn ParenCell(children: Element) -> Element {
    rsx! {
        td { class: "py-2 font-mono text-num whitespace-nowrap", {children} }
    }
}

/// 式のセル。`num` の等幅で左揃え、勝手には折り返さない。min(A, B) は「min(」の右に A と B を縦に積む。
#[component]
pub(crate) fn FormulaCell(formula: Formula) -> Element {
    rsx! {
        td { class: "py-2 pl-3 first:pl-0 first:pr-3 font-mono text-num whitespace-nowrap",
            match formula {
                Formula::Plain(f) => rsx! { {f} },
                Formula::Min(a, b) => rsx! {
                    "min("
                    span { class: "inline-block align-top", "{a}," br {} "{b})" }
                },
            }
        }
    }
}

/// 式のセル（`FormulaCell`）の式。`Min` は min(A, B) で、`FormulaCell` が A の後で折り返して B を A の下に揃える。
/// 1行の min(A, B) では式の列が広く、名前の列が語の途中で切れたため。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Formula {
    Plain(&'static str),
    Min(&'static str, &'static str),
}

/// 列ごとに小数点を揃える埋め（`numerus::point_pads`）を、行ごとに並べ直す。`rows` は（行の名前, 値の並び）
pub(crate) fn column_pads<const N: usize>(rows: &[(&str, [String; N])]) -> Vec<[usize; N]> {
    let columns: [Vec<usize>; N] = std::array::from_fn(|column| {
        point_pads(
            &rows
                .iter()
                .map(|(_, values)| &values[column])
                .collect::<Vec<_>>(),
        )
    });
    (0..rows.len())
        .map(|row| std::array::from_fn(|column| columns[column][row]))
        .collect()
}
