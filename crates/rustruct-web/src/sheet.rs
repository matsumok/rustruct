//! Tool の入力の Sheet（デザインシステムの Sheet）の部品。決め事は `docs/ui-decisions.md`「入力と表示の区別」:
//!
//! - 見出し（欄の名前）は書けないので `fill`、書けるセルは `paper` に `line` の格子
//! - 選んだセルは `ink`、読めないセルは `ng` の太い内枠（`outline-2`）で、見出しも `ng`
//! - 外の角だけ `radius-8`（ツールの入力カード）
//!
//! 単位は見出しに括弧で書く（「Lx（m）」）。

use dioxus::prelude::*;

/// 入力の Sheet の枠。中に `thead`・`tbody` を置く。列は等分（入力で幅が動かない）
#[component]
pub(crate) fn Sheet(children: Element) -> Element {
    rsx! {
        div { class: "overflow-hidden rounded-8 border border-line",
            table { class: "w-full table-fixed", {children} }
        }
    }
}

/// Sheet の列見出しの行。下に `line`
#[component]
pub(crate) fn SheetHeadRow(children: Element) -> Element {
    rsx! {
        tr { class: "border-b border-line", {children} }
    }
}

/// Sheet の本体の行。下に `line`（最後の行は外枠に任せる）
#[component]
pub(crate) fn SheetRow(children: Element) -> Element {
    rsx! {
        tr { class: "border-b border-line last:border-b-0", {children} }
    }
}

/// Sheet の見出し（行見出しか列見出し）。右に `line`（行の最後の見出しは外枠に任せる）
#[component]
pub(crate) fn SheetHead(
    label: String,
    error: bool,
    #[props(default = "row")] scope: &'static str,
) -> Element {
    rsx! {
        th {
            class: if error { "h-6 border-r border-line bg-fill px-1 text-center text-label text-ng font-normal whitespace-nowrap last:border-r-0" } else { "h-6 border-r border-line bg-fill px-1 text-center text-label text-muted font-normal whitespace-nowrap last:border-r-0" },
            scope,
            {label}
        }
    }
}

/// Sheet の書けるセル。右に `line`（行の最後のセルは外枠に任せる）
#[component]
pub(crate) fn SheetCell(children: Element) -> Element {
    rsx! {
        td { class: "border-r border-line p-0 last:border-r-0", {children} }
    }
}

/// Sheet のセルに入れる文字の欄。`numeric` なら `num` の等幅で右揃え、そうでなければ `body` で左揃え。
/// 読めない値は `ng` の内枠
#[component]
pub(crate) fn SheetInput(
    label: String,
    value: String,
    error: bool,
    #[props(default = true)] numeric: bool,
    oninput: EventHandler<FormEvent>,
) -> Element {
    let text = if numeric {
        "font-mono text-num text-right"
    } else {
        "text-body"
    };
    let frame = if error {
        "outline-2 -outline-offset-2 outline-ng"
    } else {
        "focus:outline-2 focus:-outline-offset-2 focus:outline-ink"
    };
    rsx! {
        input {
            class: "h-6 w-full bg-paper px-2 {text} {frame}",
            r#type: "text",
            inputmode: if numeric { "decimal" } else { "text" },
            "aria-label": label,
            value,
            oninput: move |e| oninput.call(e),
        }
    }
}
