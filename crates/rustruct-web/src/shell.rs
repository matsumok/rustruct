//! シェル（`GLOSSARY.md`）: どのページにも同じに出る、中身の外側。ヘッダー（サイト名・パンくず・ライトとダークの
//! 切り替え・アカウント）とメニュー。

use dioxus::prelude::*;
use dioxus_primitives::collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger};
use dioxus_primitives::dialog::{DialogContent, DialogRoot, DialogTitle};

use crate::icons::{ChevronIcon, CloseIcon, DarkIcon, LightIcon, MenuIcon, UserIcon};
use crate::{Route, WORDMARK};

/// 覚えた色（ライトかダーク）を、描く前に `<html data-theme>` に当てる。HTML は WASM より先に描かれるので、
/// ここで当てないと開くたびに一瞬 OS の色が出る（ui-decisions「画面幅」の切り替え）。
pub(crate) const APPLY_REMEMBERED_THEME: &str = r#"try{var t=localStorage.getItem("theme");if(t==="light"||t==="dark")document.documentElement.setAttribute("data-theme",t)}catch(e){}"#;

/// 今の色の反対を選んで覚える。選んでいなければ今の色は OS 設定のもの。
const TOGGLE_THEME: &str = r#"var r=document.documentElement,c=r.getAttribute("data-theme"),d=c?c==="dark":matchMedia("(prefers-color-scheme: dark)").matches,t=d?"light":"dark";r.setAttribute("data-theme",t);try{localStorage.setItem("theme",t)}catch(e){}"#;

fn toggle_theme() {
    document::eval(TOGGLE_THEME);
}

/// 全ページ共通の枠。上に固定するヘッダーと、その下の本文（ADR site-0007）。
/// 本文は画面の左右の余白（`Margin`）の内側に置く。
#[component]
pub(crate) fn Frame() -> Element {
    let route = use_route::<Route>();
    rsx! {
        document::Title { "{route.title()}" }
        document::Meta { name: "description", content: route.text().description }
        Header { route: route.clone() }
        Margin { Outlet::<Route> {} }
    }
}

/// 画面の左右の余白（tokens の `layout` の `margin`。スマホ `space-5`、iPad `space-7`、PC `space-8`）を内側に持つ箱。
/// 本文・ヘッダー・メニューの行が使い、余白の段はここにだけ書く。`class` はこの箱の残りの形（並べ方・罫・塗り）。
#[component]
fn Margin(#[props(default)] class: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "px-margin-sm md:px-margin-md lg:px-margin-lg", class: "{class}", {children} }
    }
}

/// ヘッダー。左右の余白の内側いっぱいに、左の組（サイト名）と右の組（ライトとダーク・アカウント）。
/// スマホ（`md` 未満）はサイト名を `rustruct` だけにする。四部へはどの幅でもメニューから行く（メニューのボタンは左の組の頭）。
/// 組の中の間は `space-4`。アイコンのボタンは押せる範囲を `space-3` の内側の余白で取り、その範囲の端を
/// 左右の余白の線にそろえる。
#[component]
fn Header(route: Route) -> Element {
    let mut menu_open = use_signal(|| false);
    let mut account_note = use_signal(|| false);
    rsx! {
        header { class: "sticky top-0 z-20",
            Margin { class: "flex h-header items-center justify-between border-b border-line bg-paper",
                div { class: "flex items-center gap-4",
                    // 閉じるボタンはメニューの右上にある（メニューは幕ごとヘッダーに重なる）
                    button {
                        class: "p-3 text-ink",
                        r#type: "button",
                        "aria-label": "メニューを開く",
                        "aria-expanded": if menu_open() { "true" } else { "false" },
                        onclick: move |_| menu_open.set(true),
                        MenuIcon { class: "size-5" }
                    }
                    Link { to: Route::Home {}, onclick: move |_| menu_open.set(false), Wordmark {} }
                    Breadcrumb { route: route.clone() }
                }
                div { class: "flex items-center gap-4",
                    button {
                        class: "p-3 text-ink",
                        r#type: "button",
                        "aria-label": "ライトとダークを切り替える",
                        onclick: move |_| toggle_theme(),
                        ThemeIcon {}
                    }
                    // 準備中の一言はボタンの下に重ねて出し、ヘッダーの並びを動かさない
                    div { class: "relative",
                        button {
                            class: "p-3 text-ink",
                            r#type: "button",
                            "aria-label": "アカウント",
                            "aria-expanded": if account_note() { "true" } else { "false" },
                            onclick: move |_| account_note.toggle(),
                            UserIcon { class: "size-5" }
                        }
                        if account_note() {
                            p { class: "absolute top-full right-0 whitespace-nowrap border border-line bg-paper px-3 py-2 text-label text-muted",
                                {ACCOUNT_NOTE}
                            }
                        }
                    }
                }
            }
        }
        Menu { route: route.clone(), open: menu_open }
    }
}

/// 今いる場所。wordmark の右に住所を段ごとに置き、親の段はそのページへのリンク、今いる段はリンクにしない
/// （#28）。文字は wordmark と同じ大きさの等幅で、親の段は `.matsumok.com` と同じ muted の 400。
/// スマホでは出さない（今いる場所はメニューの木で示す）。
#[component]
fn Breadcrumb(route: Route) -> Element {
    let trail = route.trail();
    if trail.is_empty() {
        return rsx! {};
    }
    let last = trail.len() - 1;
    rsx! {
        nav { class: "hidden md:block", "aria-label": "現在地",
            ol { class: "flex font-mono text-wordmark font-normal",
                for (i, (step, to)) in trail.into_iter().enumerate() {
                    li {
                        match to {
                            Some(to) => rsx! { Link { to, class: "text-muted hover:text-ink", {step} } },
                            None if i == last => rsx! { span { class: "text-ink", "aria-current": "page", {step} } },
                            None => rsx! { span { class: "text-muted", {step} } },
                        }
                    }
                }
            }
        }
    }
}

/// ログインの仕組みができるまで、アカウントのボタンが言うこと（ui-decisions「画面幅」）。
const ACCOUNT_NOTE: &str = "登録は準備中";

/// 今いるページを含む段の印（`aria-current`）。今いるページそのものには `Link` が自分で `page` を付けるので、
/// ここではその上の段の `true` だけを付ける（同じ属性を二重にしない）。
fn aria_current(here: &Route, node: &Route) -> Option<&'static str> {
    (here != node && node.holds(here)).then_some("true")
}

/// ライトとダークのアイコン。今の色のほうを出す。色は描く前に決まるが、どちらかはブラウザでしか分からないので、
/// 両方を描いて CSS で片方を隠す（初回描画と hydration で中身を変えない。ADR site-0001 の規則）。
#[component]
fn ThemeIcon() -> Element {
    rsx! {
        LightIcon { class: "size-5 dark:hidden" }
        DarkIcon { class: "hidden size-5 dark:block" }
    }
}

/// メニュー。左から出すパネルで、本文には `scrim` の幕を掛ける（ui-decisions「メニュー」）。パネルはヘッダーの高さから
/// 始まり、一番上の行の右端に × を置く（2026-10-16、本人の指示）。スマホではパネルが画面の幅いっぱいになる。
/// iPad・PC では木の1単位（`w-column`）と左右の余白の幅。
/// 振る舞いは `dioxus-primitives` の dialog に任せる（Esc と幕を押すと閉じる、フォーカスをパネルに閉じ込める、
/// `role="dialog"`）。幕がヘッダーにも重なるので、ヘッダーのボタンと「外を押すと閉じる」がぶつからない。
/// 中身は住所の木（四部と、その下のページ）。項目を選ぶと閉じる。
#[component]
fn Menu(route: Route, open: Signal<bool>) -> Element {
    let mut open = open;
    rsx! {
        DialogRoot {
            class: "fixed inset-0 z-30 bg-scrim",
            open: Some(open()),
            on_open_change: move |v| open.set(v),
            DialogContent {
                class: "fixed inset-y-0 left-0 w-full overflow-y-auto border-r border-line bg-paper md:w-auto",
                DialogTitle { class: "sr-only", "メニュー" }
                Margin { class: "flex h-header items-center justify-end border-b border-line",
                    button {
                        class: "p-3 text-ink",
                        r#type: "button",
                        "aria-label": "メニューを閉じる",
                        onclick: move |_| open.set(false),
                        CloseIcon { class: "size-5" }
                    }
                }
                Margin {
                    ul { class: "py-4 md:w-column",
                        for s in Route::sections() {
                            MenuNode { node: s, here: route.clone(), depth: 0, on_close: move |_| open.set(false) }
                        }
                    }
                }
            }
        }
    }
}

/// メニューの木の1段。見出し（と枝なら開け閉めのボタン）を左に、住所を右に置き、枝の子をその下に字下げして並べる。
/// 枝は今いるページを含むときだけ開いた形で出る。今いるページの見出しに `accent` の下線（#28）。
/// 段も子もルーターの住所一覧から導く（`Route::children`）。開け閉めは `dioxus-primitives` の collapsible。
#[component]
fn MenuNode(node: Route, here: Route, depth: usize, on_close: EventHandler<()>) -> Element {
    let children = node.children();
    let mut open = use_signal(|| node.holds(&here));
    let heading = node.text().heading;
    // 四部は heading、その下のページは body。今いるページには下線
    let class = match (depth == 0, node == here) {
        (true, false) => "text-heading",
        (true, true) => "text-heading underline decoration-accent underline-offset-4",
        (false, false) => "text-body",
        (false, true) => "text-body underline decoration-accent underline-offset-4",
    };
    // 閉じた枝の印は右向き
    let chevron = if open() {
        "size-4"
    } else {
        "size-4 -rotate-90"
    };
    let link = rsx! {
        Link {
            to: node.clone(),
            class: "py-2",
            "aria-current": aria_current(&here, &node),
            onclick: move |_| on_close.call(()),
            span { class, {heading} }
        }
    };
    let path = rsx! {
        span { class: "font-mono text-path text-muted", "{node}" }
    };
    if children.is_empty() {
        return rsx! {
            li { class: "flex items-center justify-between", {link} {path} }
        };
    }
    rsx! {
        li {
            Collapsible {
                open: Some(open()),
                on_open_change: move |v| open.set(v),
                div { class: "flex items-center justify-between",
                    div { class: "flex items-center",
                        {link}
                        CollapsibleTrigger {
                            class: "px-3 py-2 text-muted",
                            "aria-label": if open() { "{heading}を閉じる" } else { "{heading}を開く" },
                            ChevronIcon { class: chevron }
                        }
                    }
                    {path}
                }
                CollapsibleContent {
                    ul { class: "pl-5",
                        for c in children {
                            MenuNode { node: c, here: here.clone(), depth: depth + 1, on_close }
                        }
                    }
                }
            }
        }
    }
}

/// サイト名。`rustruct` は `ink` の 500、`.matsumok.com` はドットごと `muted` の 400
/// （tokens.json の `wordmark` の usage。2026-10-14 に dx serve で見て決めた）。
/// スマホのヘッダーでは `.matsumok.com` を隠し、`rustruct` だけにする。
#[component]
fn Wordmark() -> Element {
    let (name, domain) = split_wordmark();
    rsx! {
        span { class: "font-mono text-wordmark text-ink",
            {name}
            span { class: "hidden font-normal text-muted md:inline", {domain} }
        }
    }
}

/// wordmark を `rustruct` と `.matsumok.com` に分ける。
pub(crate) fn split_wordmark() -> (&'static str, &'static str) {
    let dot = WORDMARK
        .find('.')
        .expect("wordmark はドメインなのでドットを含む");
    WORDMARK.split_at(dot)
}
