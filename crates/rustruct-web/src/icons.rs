//! ヘッダーのボタンのアイコン（`docs/ui-decisions.md`「画面幅」。アイコンはヘッダーのボタンだけ）。
//!
//! IBM の Carbon（`@carbon/icons` 11.90.0、32 の格子の SVG）の path をそのまま写した。
//! Copyright IBM Corp. Apache License 2.0（写しは `src/icons/LICENSE`）。
//! 色は `currentColor` で文字の色に従い、大きさは置く側のクラス（`size-5` など）で決める。

use dioxus::prelude::*;

/// 32 の格子の SVG。`paths` は (d, transform) の組。
#[component]
fn Icon(class: &'static str, paths: &'static [(&'static str, &'static str)]) -> Element {
    rsx! {
        svg {
            class,
            view_box: "0 0 32 32",
            fill: "currentColor",
            "aria-hidden": "true",
            for (d, transform) in paths.iter().copied() {
                path { d, transform: (!transform.is_empty()).then_some(transform) }
            }
        }
    }
}

/// メニュー（`menu`）。
#[component]
pub fn MenuIcon(class: &'static str) -> Element {
    rsx! {
        Icon {
            class,
            paths: &[
                ("M4 6H28V8H4z", ""),
                ("M4 24H28V26H4z", ""),
                ("M4 12H28V14H4z", ""),
                ("M4 18H28V20H4z", ""),
            ],
        }
    }
}

/// 閉じる（`close`）。
#[component]
pub fn CloseIcon(class: &'static str) -> Element {
    rsx! {
        Icon {
            class,
            paths: &[(
                "M17.4141 16 24 9.4141 22.5859 8 16 14.5859 9.4143 8 8 9.4141 14.5859 16 8 22.5859 9.4143 24 16 17.4141 22.5859 24 24 22.5859 17.4141 16z",
                "",
            )],
        }
    }
}

/// ライト（`light`）。
#[component]
pub fn LightIcon(class: &'static str) -> Element {
    rsx! {
        Icon {
            class,
            paths: &[
                ("M15 2H17V7H15z", ""),
                ("M21.668 6.854H26.626V8.854H21.668z", "rotate(-45 24.147 7.853)"),
                ("M25 15H30V17H25z", ""),
                ("M23.147 21.668H25.147V26.626H23.147z", "rotate(-45 24.147 24.146)"),
                ("M15 25H17V30H15z", ""),
                ("M5.375 23.147H10.333V25.147H5.375z", "rotate(-45 7.853 24.146)"),
                ("M2 15H7V17H2z", ""),
                ("M6.854 5.375H8.854V10.333H6.854z", "rotate(-45 7.854 7.853)"),
                (
                    "M16,12a4,4,0,1,1-4,4,4.0045,4.0045,0,0,1,4-4m0-2a6,6,0,1,0,6,6,6,6,0,0,0-6-6Z",
                    "",
                ),
            ],
        }
    }
}

/// ダーク（`asleep`）。
#[component]
pub fn DarkIcon(class: &'static str) -> Element {
    rsx! {
        Icon {
            class,
            paths: &[(
                "M13.5025,5.4136A15.0755,15.0755,0,0,0,25.096,23.6082a11.1134,11.1134,0,0,1-7.9749,3.3893c-.1385,0-.2782.0051-.4178,0A11.0944,11.0944,0,0,1,13.5025,5.4136M14.98,3a1.0024,1.0024,0,0,0-.1746.0156A13.0959,13.0959,0,0,0,16.63,28.9973c.1641.006.3282,0,.4909,0a13.0724,13.0724,0,0,0,10.702-5.5556,1.0094,1.0094,0,0,0-.7833-1.5644A13.08,13.08,0,0,1,15.8892,4.38,1.0149,1.0149,0,0,0,14.98,3Z",
                "",
            )],
        }
    }
}

/// アカウント（`user`）。
#[component]
pub fn UserIcon(class: &'static str) -> Element {
    rsx! {
        Icon {
            class,
            paths: &[
                ("M16,4a5,5,0,1,1-5,5,5,5,0,0,1,5-5m0-2a7,7,0,1,0,7,7A7,7,0,0,0,16,2Z", ""),
                (
                    "M26,30H24V25a5,5,0,0,0-5-5H13a5,5,0,0,0-5,5v5H6V25a7,7,0,0,1,7-7h6a7,7,0,0,1,7,7Z",
                    "",
                ),
            ],
        }
    }
}
