//! Tool の入力の Sheet（デザインシステムの Sheet）の部品。決め事は `docs/ui-decisions.md`「入力と表示の区別」:
//!
//! - 見出し（欄の名前）は書けないので `fill`、書けるセルは `paper` に `line` の格子
//! - 選んだセルは `ink`、読めないセルは `ng` の太い内枠（`outline-2`）で、見出しも `ng`
//! - 外の角だけ `radius-8`（ツールの入力カード）
//!
//! 単位は見出しに括弧で書く（「Lx（m）」）。

use dioxus::prelude::*;
use dioxus_primitives::checkbox::{Checkbox, CheckboxIndicator, CheckboxState};
use dioxus_primitives::select::{Select, SelectList, SelectOption, SelectTrigger};

use crate::icons::{CheckIcon, ChevronIcon};

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

/// Sheet のセルに入れるチェックの欄。振る舞いは `dioxus-primitives` の checkbox（`role="checkbox"` と
/// `aria-checked`、Space で切り替え）。セル全体が押せるボタンで、フォーカスは書けるセルと同じ `ink` の内枠。
/// 中の `size-4` の箱は、書ける器の `line-strong` の枠で、オンのとき `ink` で塗って `paper` の印を出す
#[component]
pub(crate) fn SheetCheck(label: String, checked: bool, onchange: EventHandler<bool>) -> Element {
    rsx! {
        Checkbox {
            class: "group flex h-6 w-full items-center bg-paper px-2 focus:outline-2 focus:-outline-offset-2 focus:outline-ink",
            aria_label: label,
            checked: Some(if checked { CheckboxState::Checked } else { CheckboxState::Unchecked }),
            on_checked_change: move |state: CheckboxState| onchange.call(state.into()),
            span { class: "flex size-4 items-center justify-center border border-line-strong bg-paper text-paper group-aria-checked:border-ink group-aria-checked:bg-ink",
                CheckboxIndicator { class: "flex", CheckIcon { class: "size-3" } }
            }
        }
    }
}

/// 選択の欄のリストを閉じたとき、フォーカスがリストの中にあればボタンに戻す（Enter・Space・Esc・選択肢を
/// 押して閉じたとき。素の `select` と同じく、選んだ後も欄に留まる）。primitives の select は戻さず、閉じた
/// リストが消えるとフォーカスが `body` に落ちる。外を押して閉じたときは、フォーカスはもう移った先にあるので動かさない。
/// ボタンはリストを `aria-controls` で指す（primitives が付ける）。リストは閉じる処理の中で読み、フォーカスは
/// `spawn` で処理を抜けてから移す（処理の中で移すと、フォーカスの抜けた選択肢の blur が入れ子で走り、WASM が止まる）。
/// ブラウザの外（SSG とテスト）では何もしない
fn return_focus_to_trigger() {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::HtmlElement;
        use web_sys::wasm_bindgen::JsCast;

        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Some(list) = document
            .active_element()
            .and_then(|focused| focused.closest("[role=listbox]").ok().flatten())
        else {
            return;
        };
        let Ok(controls) = document.query_selector_all("[aria-controls]") else {
            return;
        };
        let trigger = (0..controls.length())
            .filter_map(|index| controls.item(index))
            .filter_map(|node| node.dyn_into::<HtmlElement>().ok())
            .find(|element| element.get_attribute("aria-controls") == Some(list.id()));
        if let Some(trigger) = trigger {
            spawn(async move {
                _ = trigger.focus();
            });
        }
    }
}

/// Sheet のセルに入れる選択の欄。選択肢は（記号, 名前）の並びで、値は記号。振る舞いは `dioxus-primitives` の
/// select（リストを開くボタン、矢印と Enter と Esc、打った文字で選択肢に移る、読み上げの `listbox`）。
/// ボタンは書けるセルと同じ形で、選んだ名前と開く印（`chevron--down`）を出す。リストはセルの真下に重ね、
/// 選んだ選択肢を `ink` で塗り（量の切替と同じ）、フォーカスは書けるセルと同じ `ink` の内枠。
/// リストは位置を指定しない `absolute` で、置かれる位置はボタンの下のまま、包む箱は Sheet の外になる。
/// Sheet の角を丸める `overflow-hidden` に切られないのはこのため（セルや Sheet に `relative` を付けない。
/// `inputs.rs` のテストが見る）
#[component]
pub(crate) fn SheetSelect(
    label: String,
    options: Vec<(char, &'static str)>,
    selected: ReadSignal<char>,
    onchange: EventHandler<char>,
) -> Element {
    let name = options
        .iter()
        .find(|(symbol, _)| *symbol == selected())
        .map(|(_, name)| *name);
    let value = use_memo(move || Some(selected()));
    rsx! {
        Select::<char> {
            value: ReadSignal::from(value),
            on_value_change: move |symbol: Option<char>| {
                if let Some(symbol) = symbol {
                    onchange.call(symbol);
                }
            },
            on_open_change: move |open: bool| {
                if !open {
                    return_focus_to_trigger();
                }
            },
            SelectTrigger {
                class: "flex h-6 w-full items-center justify-between gap-1 bg-paper px-2 text-body focus:outline-2 focus:-outline-offset-2 focus:outline-ink",
                aria_label: label,
                // 収まらない名前は素の select と同じく折り返さずに切る
                span { class: "min-w-0 overflow-hidden whitespace-nowrap", {name} }
                ChevronIcon { class: "size-4 text-muted" }
            }
            SelectList { class: "absolute z-10 border border-line bg-paper focus:outline-2 focus:-outline-offset-2 focus:outline-ink",
                for (index , (symbol , name)) in options.into_iter().enumerate() {
                    SelectOption::<char> {
                        class: "flex h-6 items-center whitespace-nowrap px-2 text-body focus:outline-2 focus:-outline-offset-2 focus:outline-ink aria-selected:bg-ink aria-selected:text-paper aria-selected:focus:outline-paper",
                        index,
                        value: symbol,
                        text_value: name.to_string(),
                        {name}
                    }
                }
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
