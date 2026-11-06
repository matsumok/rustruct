//! ページの中身: トップ、ページの一覧、下層ページの見出しと本文の枠、まだ中身の無いページ。

use dioxus::prelude::*;

use crate::Route;
use crate::route::split_wordmark;

/// トップ。`display` のサイトの住所と、四部の一覧（名前＋一行＋パス）。
/// スマホのヘッダーが `rustruct` だけになっても、ここで住所が全部見える。狭い画面ではドットの前で折り返す。
#[component]
pub(crate) fn Home() -> Element {
    let (name, host) = split_wordmark();
    rsx! {
        main { class: "mx-auto max-w-page lg:max-w-page-lg",
            p { class: "text-display py-8",
                {name}
                wbr {}
                span { class: "text-muted", {host} }
            }
            RouteList { routes: Route::sections().collect::<Vec<_>>() }
        }
    }
}

/// ページの一覧。各項目は「パス＋名前＋一行」で、押すとそのページ。箱で囲まず、
/// スマホは1列、`md` から1単位（`w-column`）の列を `space-4` の溝で2つ。Data の列と同じ線に並ぶ。
#[component]
pub(crate) fn RouteList(routes: Vec<Route>) -> Element {
    rsx! {
        ul { class: "grid grid-cols-1 md:w-fit md:grid-cols-2 gap-x-4 gap-y-6",
            for route in routes {
                li { class: "md:w-column",
                    Link { to: route.clone(), class: "block",
                        span { class: "block font-mono text-path text-muted", "{route}" }
                        span { class: "block text-heading", {route.text().heading} }
                        span { class: "block text-body", {route.text().description} }
                    }
                }
            }
        }
    }
}

/// Studies / Notes の空ページ。
#[component]
pub(crate) fn SectionPage() -> Element {
    rsx! {
        Page { route: use_route::<Route>(), "まだ何もない。" }
    }
}

/// 下層ページ。`title` の見出し（ページ名）、本文の順。サイドバーは無い。
/// 本文の箱は最大2単位（`max-w-page`）で、左右の余白の内側で中央に置く。
/// 住所は本文に出さない。iPad・PC はヘッダーのパンくず、スマホはメニューの木で示す（#28）。
#[component]
pub(crate) fn Page(route: Route, children: Element) -> Element {
    rsx! {
        // 見出しと本文の間は space-7（ページの見出しも一つの区切りなので、セクションの間と同じ。12 では狭く見えた。
        // 2026-10-15、20・28・40 を撮り比べて本人が決めた）
        main { class: "mx-auto flex max-w-page flex-col gap-7 py-7 lg:max-w-page-lg",
            h1 { class: "text-title", {route.text().heading} }
            div { class: "text-body", {children} }
        }
    }
}

/// root のための場所。住所を予約するだけで、Access のポリシーはまだ掛けない（Issue #11）。
#[component]
pub(crate) fn RootPage() -> Element {
    let route = Route::Root {};
    rsx! {
        Page { route: route.clone(), "{route.text().description}まだ何もない。" }
    }
}

/// 存在しない住所。同じ枠の中で「ない」ことを一文で示す。
///
/// 静的な 1 ファイル（`NOT_FOUND_PATH`）がどの住所でも届くので、求められた住所は出さない（ADR site-0001 の規則）。
#[component]
pub(crate) fn NotFound(segments: Vec<String>) -> Element {
    let route = Route::NotFound { segments };
    rsx! {
        Page { route: route.clone(), {route.text().description} }
    }
}
