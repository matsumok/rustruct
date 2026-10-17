//! PROTOTYPE（使い捨て。ブランチ `proto-primitives` だけ）。
//! 問い: `dioxus-primitives` の骨（挙動と ARIA）に Tailwind の役割クラスだけで皮を着せられるか。
//! WASM の増分と SSG + hydration が成り立つかを `/data/concrete` で測る。本線には入れない。

use dioxus::prelude::*;
use dioxus_primitives::checkbox::{self, CheckboxState};
use dioxus_primitives::combobox;

/// Fc の候補。
const FC_OPTIONS: [&str; 7] = ["18", "21", "24", "27", "30", "33", "36"];

/// 試作の塊。チェックボックスで候補の表示を切り替え、コンボボックスで Fc を選ぶ。状態は下に全部出す。
#[component]
pub(crate) fn Proto(fc: Signal<String>) -> Element {
    let show = use_signal(|| true);
    rsx! {
        div { class: "flex flex-col gap-3 border border-line p-3",
            p { class: "text-label text-muted", "PROTOTYPE dioxus-primitives" }
            Checkbox { checked: show, "Fc の候補を出す" }
            if show() {
                FcCombobox { fc }
            }
            p { class: "font-mono text-path text-muted", "state: show={show} fc={fc}" }
        }
    }
}

/// 骨は `checkbox::Checkbox`（role=checkbox、aria-checked、Space で切替、hidden input）。
/// 皮: 塗り＝書けない、枠＝書ける。未選択は `paper` の地に `line-strong` の枠、選択は `ink` で塗って
/// `on-ink` の小さな四角。アイコンは使わない。
#[component]
fn Checkbox(checked: Signal<bool>, children: Element) -> Element {
    let state = use_memo(move || {
        Some(if checked() {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        })
    });
    rsx! {
        label { class: "flex items-center gap-2 text-body",
            checkbox::Checkbox {
                class: "flex size-4 items-center justify-center rounded-0 border border-line-strong bg-paper data-[state=checked]:border-ink data-[state=checked]:bg-ink",
                checked: state,
                on_checked_change: move |s: CheckboxState| checked.set(s.into()),
                checkbox::CheckboxIndicator { class: "block size-2 bg-on-ink" }
            }
            {children}
        }
    }
}

/// 骨は `combobox::Combobox`（role=combobox、aria-expanded、矢印キー、絞り込み）。
/// 皮: 入力は今の Fc の `input` と同じクラス。一覧は `paper` の地に `line` の枠、強調行は `fill`。
#[component]
fn FcCombobox(fc: Signal<String>) -> Element {
    let value = use_memo(move || Some(fc()));
    rsx! {
        combobox::Combobox::<String> {
            class: "relative inline-block",
            value: Some(ReadSignal::from(value)),
            on_value_change: move |v: Option<String>| {
                if let Some(v) = v {
                    fc.set(v);
                }
            },
            combobox::ComboboxInput {
                class: "w-32 rounded-8 border border-line-strong bg-paper px-3 py-2 font-mono text-num text-right",
                placeholder: "Fc",
            }
            combobox::ComboboxList {
                class: "absolute left-0 top-full z-10 w-32 border border-line bg-paper data-[state=closed]:hidden",
                for (index, option) in FC_OPTIONS.iter().enumerate() {
                    combobox::ComboboxOption::<String> {
                        value: option.to_string(),
                        index,
                        class: "px-3 py-1 font-mono text-num text-right data-[highlighted=true]:bg-fill data-[selected=true]:text-accent",
                        "{option}"
                    }
                }
                combobox::ComboboxEmpty { class: "px-3 py-1 text-label text-muted", "候補なし" }
            }
        }
    }
}
