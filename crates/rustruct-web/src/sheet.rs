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
            class: "h-6 border-r border-line bg-fill px-1 text-center text-label text-muted font-normal whitespace-nowrap last:border-r-0 error:text-ng",
            "data-state": error.then_some("error"),
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

/// Sheet のセルに入れるチェックの欄。`size-4` の素のチェックボックスを `ink` で、セルの高さの中央に
#[component]
pub(crate) fn SheetCheck(
    label: String,
    checked: bool,
    onchange: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        div { class: "flex h-6 items-center px-2",
            input {
                class: "size-4 accent-ink",
                r#type: "checkbox",
                "aria-label": label,
                checked,
                onchange: move |e| onchange.call(e),
            }
        }
    }
}

/// Sheet のセルに入れる選択の欄。素の `select` で、選択肢は（記号, 名前）の並び。値は記号
#[component]
pub(crate) fn SheetSelect(
    label: String,
    options: Vec<(char, &'static str)>,
    selected: char,
    onchange: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        select {
            class: "h-6 w-full bg-paper px-2 text-body focus:outline-2 focus:-outline-offset-2 focus:outline-ink",
            "aria-label": label,
            onchange: move |e| onchange.call(e),
            for (symbol , name) in options {
                option { value: "{symbol}", selected: selected == symbol, {name} }
            }
        }
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
    rsx! {
        input {
            // 同じ要素の class: は Dioxus がつないで出す。違う部分（数値か文字か）だけを if で選ぶ
            class: "h-6 w-full bg-paper px-2 focus:outline-2 focus:-outline-offset-2 focus:outline-ink aria-invalid:outline-2 aria-invalid:-outline-offset-2 aria-invalid:outline-ng",
            class: if numeric { "font-mono text-num text-right" } else { "text-body" },
            r#type: "text",
            inputmode: if numeric { "decimal" } else { "text" },
            "aria-label": label,
            "aria-invalid": error.then_some("true"),
            value,
            oninput: move |e| oninput.call(e),
        }
    }
}
