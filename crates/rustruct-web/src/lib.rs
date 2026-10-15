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

/// Data: 材料や規格の値のページ。
mod data;

/// ヘッダーのボタンのアイコン（Carbon）。
mod icons;
pub use data::{AllowableRow, ConcreteReadout};
use data::{ConcretePage, DataIndex, RebarPage};
use icons::{CloseIcon, DarkIcon, LightIcon, MenuIcon, UserIcon};
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

    /// このページが入る四部の一つ（`/data/concrete` なら Data）。トップ・root・not-found は無し。
    /// not-found はどの住所でも同じ 1 ファイルなので、`/tools/nothing` でも部に入れない（ADR-0001 の規則）。
    fn section(&self) -> Option<Route> {
        if matches!(self, Route::NotFound { .. }) {
            return None;
        }
        let path = self.to_string();
        Route::sections().find(|section| {
            let base = section.to_string();
            path == base || path.starts_with(&format!("{base}/"))
        })
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

/// ヘッダー。左右の余白の内側いっぱいに、左の組（サイト名）、中央の四部、右の組（ページ名・ライトとダーク・
/// アカウント）。スマホ（`md` 未満）はサイト名を `rustruct` だけにし、四部だけをメニューにしまう（メニューのボタンは
/// 左の組の頭）。組の中の間は `space-4`。アイコンのボタンは押せる範囲を `space-3` の内側の余白で取り、
/// その範囲の端を左右の余白の線にそろえる。
#[component]
fn Header(route: Route) -> Element {
    let mut menu_open = use_signal(|| false);
    let mut account_note = use_signal(|| false);
    let page_name = (route != Route::Home {}).then(|| route.text().heading);
    rsx! {
        // sticky は位置を持つので、中央の四部（absolute）はヘッダーを基準に置かれる
        header { class: "sticky top-0 z-20 flex h-8 items-center justify-between border-b border-line bg-paper px-5 md:px-7 lg:px-8",
            div { class: "flex items-center gap-4",
                button {
                    class: "p-3 text-ink md:hidden",
                    r#type: "button",
                    "aria-label": if menu_open() { "メニューを閉じる" } else { "メニューを開く" },
                    "aria-expanded": if menu_open() { "true" } else { "false" },
                    onclick: move |_| menu_open.toggle(),
                    if menu_open() {
                        CloseIcon { class: "size-5" }
                    } else {
                        MenuIcon { class: "size-5" }
                    }
                }
                Link { to: Route::Home {}, onclick: move |_| menu_open.set(false), Wordmark {} }
            }
            // 四部はヘッダーの中央。左右の組の幅に引かれないよう、並びから外して置く
            nav { class: "hidden md:absolute md:left-1/2 md:flex md:-translate-x-1/2 md:items-center md:gap-4",
                for section in Route::sections() {
                    SectionLink { route: route.clone(), section }
                }
            }
            div { class: "flex min-w-0 items-center gap-4",
                // 部のページそのものの名前は、iPad・PC では下線の付いた部のボタンと同じなので出さない
                span {
                    class: if Route::sections().any(|s| s == route) { "truncate text-body text-muted md:hidden" } else { "truncate text-body text-muted" },
                    {page_name}
                }
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
        if menu_open() {
            Menu { route: route.clone(), on_close: move |_| menu_open.set(false) }
        }
    }
}

/// ログインの仕組みができるまで、アカウントのボタンが言うこと（ui-decisions「画面幅」）。
const ACCOUNT_NOTE: &str = "登録は準備中";

/// 今いる部の印（`aria-current`）。部のページそのものには `Link` が自分で `page` を付けるので、
/// ここではその下のページの `true` だけを付ける（同じ属性を二重にしない）。
fn aria_current(route: &Route, section: &Route) -> Option<&'static str> {
    (route != section && route.section().as_ref() == Some(section)).then_some("true")
}

/// ヘッダーの四部のボタン。今いる部には `accent` の下線（差し色の使い場所の一つ）。
#[component]
fn SectionLink(route: Route, section: Route) -> Element {
    let here = route.section().as_ref() == Some(&section);
    rsx! {
        Link {
            to: section.clone(),
            class: if here { "text-body text-ink underline decoration-accent underline-offset-4" } else { "text-body text-ink" },
            "aria-current": aria_current(&route, &section),
            {section.text().heading}
        }
    }
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

/// スマホのメニュー。ヘッダーは残し、その下の画面をまるごと `paper` で覆う（ui-decisions「画面幅」）。
/// 中身は四部（名前とパス。今いる部に下線）だけ。ライトとダーク・アカウントはヘッダーにある。項目を選ぶと閉じる。
#[component]
fn Menu(route: Route, on_close: EventHandler<()>) -> Element {
    let here = route.section();
    rsx! {
        div { class: "fixed inset-x-0 top-8 bottom-0 z-10 overflow-y-auto bg-paper px-5 md:hidden",
            ul { class: "py-4",
                for s in Route::sections() {
                    li {
                        Link {
                            to: s.clone(),
                            class: "flex items-baseline justify-between py-3",
                            "aria-current": aria_current(&route, &s),
                            onclick: move |_| on_close.call(()),
                            span { class: if here.as_ref() == Some(&s) { "text-heading underline decoration-accent underline-offset-4" } else { "text-heading" },
                                {s.text().heading}
                            }
                            span { class: "font-mono text-path text-muted", "{s}" }
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

/// 住所で組む下層ページ。伸びた住所、`title` の見出し（ページ名）、本文の順。サイドバーもパンくずも無い。
/// 本文の箱は最大2単位（`max-w-page`）で、左右の余白の内側で中央に置く。
/// 住所は長い一語でも折り返す（not-found はどんな住所でも受ける）。
/// `with_path` を倒すと住所の行を空にする（not-found が起動前に使う）。
#[component]
fn Page(route: Route, #[props(default = true)] with_path: bool, children: Element) -> Element {
    rsx! {
        main { class: "mx-auto flex max-w-page flex-col gap-3 py-7 lg:max-w-page-lg",
            // 住所と見出しの組（間は space-2）。本文との間は space-3
            div { class: "flex flex-col gap-2",
                p { class: "font-mono text-path text-muted break-all",
                    {with_path.then(|| route.to_string())}
                }
                h1 { class: "text-title", {route.text().heading} }
            }
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
/// 静的な 1 ファイル（`NOT_FOUND_PATH`）がどの住所でも届くので、求められた住所はブラウザでしか
/// 決まらない。初回描画には入れず、起動後に `use_effect` で出す（ADR-0001 の規則）。
#[component]
fn NotFound(segments: Vec<String>) -> Element {
    let route = Route::NotFound { segments };
    let mut started = use_signal(|| false);
    use_effect(move || started.set(true));
    rsx! {
        Page { route: route.clone(), with_path: started(), {route.text().description} }
    }
}
