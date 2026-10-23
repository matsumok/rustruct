//! サイトの画面。Dioxus fullstack で、`web`（ブラウザ）と `server`（SSG）の feature で切り替える。
//! ネイティブの `cargo test` では feature なしで HTML を描き、ブラウザも WASM も使わない。
//!
//! 見た目はすべて役割名のクラス（`text-ink`、`text-wordmark`、`px-5` …）で書く。値は
//! `docs/design/tokens.json` にだけあり、`theme.css` を経て Tailwind のクラスになる。

use dioxus::prelude::*;

/// tokens.json → @theme の生成器。build.rs が使う。lib には test のときだけ入れて継ぎ目 B を検査する。
#[cfg(test)]
mod theme;

/// 継ぎ目 A: パス → HTML 文字列。ネイティブだけ（ブラウザは hydration で同じ App を描く）。
#[cfg(not(target_arch = "wasm32"))]
mod prerender;
#[cfg(not(target_arch = "wasm32"))]
pub use prerender::render_path;

/// SSG: 書き出す住所の列挙と、書く場所の設定。
mod ssg;

/// 計算のページ（Data と Tool）の表示用の表と列の部品。
mod table;

/// Data: 材料や規格の値のページ。
mod data;

/// ヘッダーのボタンのアイコン（Carbon）。
mod icons;
pub use data::{AllowableRow, ConcreteReadout};
use data::{ConcretePage, DataIndex, RebarPage};
use dioxus_primitives::collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger};
use dioxus_primitives::dialog::{DialogContent, DialogRoot, DialogTitle};
use icons::{ChevronIcon, CloseIcon, DarkIcon, LightIcon, MenuIcon, UserIcon};
#[cfg(feature = "server")]
pub use ssg::serve_config;
pub use ssg::{NOT_FOUND_PATH, static_paths};

include!(concat!(env!("OUT_DIR"), "/fonts.rs"));

/// ヘッダーに置く wordmark の文字。サイトの住所そのもの。
pub const WORDMARK: &str = "rustruct.matsumok.com";

/// dx 内蔵の Tailwind CLI が `tailwind.css` から出力した CSS。
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// サイトの住所。全ページが `Frame`（ヘッダーと本文の枠）の中に入る。
#[derive(Routable, Clone, PartialEq, Debug)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Frame)]
        #[route("/")]
        Home {},
        #[route("/tools", SectionPage)]
        Tools {},
        #[route("/data", DataIndex)]
        Data {},
        #[route("/data/concrete", ConcretePage)]
        DataConcrete {},
        #[route("/data/rebar", RebarPage)]
        DataRebar {},
        #[route("/studies", SectionPage)]
        Studies {},
        #[route("/notes", SectionPage)]
        Notes {},
        #[route("/root", RootPage)]
        Root {},
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}

/// ルートが載る一覧。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Listing {
    /// トップの一覧（四部）
    Home,
    /// `/data` の一覧
    Data,
    /// どこにも載らない
    Nowhere,
}

/// ページの文言。見出し（ページ名）と一行の説明。`<title>` と description の meta、
/// 一覧に載るページ（四部、Data の各ページ）なら、一覧の名前と一行も、ここから導く。
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PageText {
    pub heading: &'static str,
    pub description: &'static str,
}

impl Route {
    /// ルートごとの文言。足すときはここに 1 行書けば、見出し・`<title>`・meta（一覧に載るページなら一覧も）に載る。
    pub fn text(&self) -> PageText {
        let (heading, description) = match self {
            Route::Home {} => (
                "Home",
                "構造設計者向けの計算ツールと、数学的な可視化・書きもの。",
            ),
            Route::Tools {} => ("Tools", "構造計算の入力・計算・判定。"),
            Route::Data {} => ("Data", "構造設計で参照する材料や規格の値。"),
            Route::DataConcrete {} => (
                "コンクリート",
                "Fc から単位体積重量・ヤング係数・許容応力度を求める。",
            ),
            Route::DataRebar {} => (
                "鉄筋",
                "異形棒鋼の呼び名ごとの公称直径・周長・断面積・単位質量と、種類ごとの許容応力度。",
            ),
            Route::Studies {} => ("Studies", "数学やかたちを眺めて考えるための可視化。"),
            Route::Notes {} => ("Notes", "構造や数学についての書きもの。"),
            Route::Root {} => ("root", "root のための場所。"),
            Route::NotFound { .. } => ("Not found", "この住所には何もない。"),
        };
        PageText {
            heading,
            description,
        }
    }

    /// どの一覧に載るか。ルートを足したらここで決める（網羅的な match なので、決め忘れはコンパイルが止める）。
    fn listing(&self) -> Listing {
        match self {
            Route::Tools {} | Route::Data {} | Route::Studies {} | Route::Notes {} => Listing::Home,
            Route::DataConcrete {} | Route::DataRebar {} => Listing::Data,
            Route::Home {} | Route::Root {} | Route::NotFound { .. } => Listing::Nowhere,
        }
    }

    /// トップに並べる四部（Tools / Data / Studies / Notes）。ルーターの住所一覧から導くので、
    /// 一覧とルーターを別々に書かない。
    pub fn sections() -> impl Iterator<Item = Route> {
        Route::listed_on(Listing::Home)
    }

    /// `/data` に並べるページ。四部と同じく、ルーターの住所一覧から導く。
    pub fn data_pages() -> impl Iterator<Item = Route> {
        Route::listed_on(Listing::Data)
    }

    fn listed_on(listing: Listing) -> impl Iterator<Item = Route> {
        Route::static_routes()
            .into_iter()
            .filter(move |route| route.listing() == listing)
    }

    /// `<title>`。「ページ名 · サイト名」。
    pub fn title(&self) -> String {
        format!("{} · {WORDMARK}", self.text().heading)
    }

    /// ヘッダーのパンくずの段。住所を「/」ごとに区切った各段と、押すと行くページ（今いる段と、ページの無い段は
    /// None）。行き先はルーターの住所一覧から引くので、一覧を別に書かない。トップは段が無い。not-found は
    /// どの住所でも同じ 1 ファイルなので、住所を出さない（ADR-0001 の規則）。
    fn trail(&self) -> Vec<(String, Option<Route>)> {
        if matches!(self, Route::Home {} | Route::NotFound { .. }) {
            return Vec::new();
        }
        let path = self.to_string();
        let mut prefix = String::new();
        let mut steps = path.split('/').filter(|s| !s.is_empty()).peekable();
        let mut trail = Vec::new();
        while let Some(step) = steps.next() {
            prefix = format!("{prefix}/{step}");
            let to = steps.peek().and_then(|_| {
                Route::static_routes()
                    .into_iter()
                    .find(|r| r.to_string() == prefix)
            });
            trail.push((format!("/{step}"), to));
        }
        trail
    }

    /// 住所が一段だけ深いページ（`/data` なら `/data/concrete` と `/data/rebar`）。メニューの木の枝。
    /// ルーターの住所一覧から導く。
    fn children(&self) -> Vec<Route> {
        let base = format!("{self}/");
        Route::static_routes()
            .into_iter()
            .filter(|r| {
                r.to_string()
                    .strip_prefix(&base)
                    .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
            })
            .collect()
    }

    /// `other` がこのページか、その下のページか。not-found はどこにも入らない（ADR-0001 の規則）。
    fn holds(&self, other: &Route) -> bool {
        if matches!(other, Route::NotFound { .. }) {
            return false;
        }
        let (base, path) = (self.to_string(), other.to_string());
        path == base || path.starts_with(&format!("{base}/"))
    }
}

/// 覚えた色（ライトかダーク）を、描く前に `<html data-theme>` に当てる。HTML は WASM より先に描かれるので、
/// ここで当てないと開くたびに一瞬 OS の色が出る（ui-decisions「画面幅」の切り替え）。
const APPLY_REMEMBERED_THEME: &str = r#"try{var t=localStorage.getItem("theme");if(t==="light"||t==="dark")document.documentElement.setAttribute("data-theme",t)}catch(e){}"#;

/// 今の色の反対を選んで覚える。選んでいなければ今の色は OS 設定のもの。
const TOGGLE_THEME: &str = r#"var r=document.documentElement,c=r.getAttribute("data-theme"),d=c?c==="dark":matchMedia("(prefers-color-scheme: dark)").matches,t=d?"light":"dark";r.setAttribute("data-theme",t);try{localStorage.setItem("theme",t)}catch(e){}"#;

fn toggle_theme() {
    document::eval(TOGGLE_THEME);
}

/// サイト全体のルート。フォントと CSS の読込と、ルーター。
#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        document::Stylesheet { href: GOOGLE_FONTS_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Script { {APPLY_REMEMBERED_THEME} }
        Router::<Route> {}
    }
}

/// 全ページ共通の枠。上に固定するヘッダーと、その下の本文（`docs/adr/0007`）。
/// 本文はページの左右の余白（スマホ `space-5`、iPad `space-7`、PC `space-8`）の内側に置く。
#[component]
fn Frame() -> Element {
    let route = use_route::<Route>();
    rsx! {
        document::Title { "{route.title()}" }
        document::Meta { name: "description", content: route.text().description }
        Header { route: route.clone() }
        div { class: "px-5 md:px-7 lg:px-8", Outlet::<Route> {} }
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
        header { class: "sticky top-0 z-20 flex h-8 items-center justify-between border-b border-line bg-paper px-5 md:px-7 lg:px-8",
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
/// 両方を描いて CSS で片方を隠す（初回描画と hydration で中身を変えない。ADR-0001 の規則）。
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
                div { class: "flex h-8 items-center justify-end border-b border-line px-5 md:px-7 lg:px-8",
                    button {
                        class: "p-3 text-ink",
                        r#type: "button",
                        "aria-label": "メニューを閉じる",
                        onclick: move |_| open.set(false),
                        CloseIcon { class: "size-5" }
                    }
                }
                ul { class: "px-5 py-4 md:w-column md:box-content md:px-7 lg:px-8",
                    for s in Route::sections() {
                        MenuNode { node: s, here: route.clone(), depth: 0, on_close: move |_| open.set(false) }
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
                            class: "p-3 text-muted",
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
fn split_wordmark() -> (&'static str, &'static str) {
    let dot = WORDMARK
        .find('.')
        .expect("wordmark はドメインなのでドットを含む");
    WORDMARK.split_at(dot)
}

/// トップ。`display` のサイトの住所と、四部の一覧（名前＋一行＋パス）。
/// スマホのヘッダーが `rustruct` だけになっても、ここで住所が全部見える。狭い画面ではドットの前で折り返す。
#[component]
fn Home() -> Element {
    let (name, domain) = split_wordmark();
    rsx! {
        main { class: "mx-auto max-w-page lg:max-w-page-lg",
            p { class: "text-display py-8",
                {name}
                wbr {}
                span { class: "text-muted", {domain} }
            }
            RouteList { routes: Route::sections().collect::<Vec<_>>() }
        }
    }
}

/// ページの一覧。各項目は「パス＋名前＋一行」で、押すとそのページ。箱で囲まず、
/// スマホは1列、`md` から1単位（`w-column`）の列を `space-4` の溝で2つ。Data の列と同じ線に並ぶ。
#[component]
fn RouteList(routes: Vec<Route>) -> Element {
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

/// Tools / Studies / Notes の空ページ。
#[component]
fn SectionPage() -> Element {
    rsx! {
        Page { route: use_route::<Route>(), "まだ何もない。" }
    }
}

/// 下層ページ。`title` の見出し（ページ名）、本文の順。サイドバーは無い。
/// 本文の箱は最大2単位（`max-w-page`）で、左右の余白の内側で中央に置く。
/// 住所は本文に出さない。iPad・PC はヘッダーのパンくず、スマホはメニューの木で示す（#28）。
#[component]
fn Page(route: Route, children: Element) -> Element {
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
fn RootPage() -> Element {
    let route = Route::Root {};
    rsx! {
        Page { route: route.clone(), "{route.text().description}まだ何もない。" }
    }
}

/// 存在しない住所。同じ枠の中で「ない」ことを一文で示す。
///
/// 静的な 1 ファイル（`NOT_FOUND_PATH`）がどの住所でも届くので、求められた住所は出さない（ADR-0001 の規則）。
#[component]
fn NotFound(segments: Vec<String>) -> Element {
    let route = Route::NotFound { segments };
    rsx! {
        Page { route: route.clone(), {route.text().description} }
    }
}
