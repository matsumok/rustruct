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
pub use data::{AllowableRow, ConcreteReadout};
use data::{ConcretePage, DataIndex, RebarPage};
#[cfg(feature = "server")]
pub use ssg::serve_config;
pub use ssg::{NOT_FOUND_PATH, static_paths};

include!(concat!(env!("OUT_DIR"), "/fonts.rs"));

/// 上のバーに置く wordmark の文字。サイトの住所そのもの。
pub const WORDMARK: &str = "rustruct.matsumok.com";

/// dx 内蔵の Tailwind CLI が `tailwind.css` から出力した CSS。
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// サイトの住所。全ページが `Frame`（上のバーの wordmark）の中に入る。
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
}

/// サイト全体のルート。フォントと CSS の読込と、ルーター。
#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        document::Stylesheet { href: GOOGLE_FONTS_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

/// 全ページ共通の枠。上のバーの wordmark（押すとトップ）と、その下にページ本体。
#[component]
fn Frame() -> Element {
    let route = use_route::<Route>();
    rsx! {
        document::Title { "{route.title()}" }
        document::Meta { name: "description", content: route.text().description }
        header { class: "px-5 py-4",
            Link { to: Route::Home {}, Wordmark {} }
        }
        Outlet::<Route> {}
    }
}

/// サイト名。`rustruct` は `ink` の 500、`.matsumok.com` はドットごと `muted` の 400
/// （tokens.json の `wordmark` の usage。2026-10-14 に dx serve で見て決めた）。
#[component]
fn Wordmark() -> Element {
    let dot = WORDMARK
        .find('.')
        .expect("wordmark はドメインなのでドットを含む");
    let (name, domain) = WORDMARK.split_at(dot);
    rsx! {
        span { class: "font-mono text-wordmark text-ink",
            {name}
            span { class: "text-muted font-normal", {domain} }
        }
    }
}

/// トップ。`display` の一文と、四部の一覧（名前＋一行＋パス）。
#[component]
fn Home() -> Element {
    // 見出しの一文の句点だけ accent（docs/design/README.md「差し色」）
    let sentence = "構造を計算し、かたちを眺める。";
    let words = sentence
        .strip_suffix('。')
        .expect("トップの一文は句点で終わる");
    let period = &sentence[words.len()..];
    rsx! {
        main { class: "px-5",
            p { class: "text-display py-8",
                {words}
                span { class: "text-accent", {period} }
            }
            RouteList { routes: Route::sections().collect::<Vec<_>>() }
        }
    }
}

/// ページの一覧。各項目は「パス＋名前＋一行」で、押すとそのページ。箱で囲まず、グリッドは2列まで。
#[component]
fn RouteList(routes: Vec<Route>) -> Element {
    rsx! {
        ul { class: "grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-6",
            for route in routes {
                li {
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
/// 住所は長い一語でも折り返す（not-found はどんな住所でも受ける）。
/// `with_path` を倒すと住所の行を空にする（not-found が起動前に使う）。
#[component]
fn Page(route: Route, #[props(default = true)] with_path: bool, children: Element) -> Element {
    rsx! {
        main { class: "px-5 py-7",
            p { class: "font-mono text-path text-muted break-all",
                {with_path.then(|| route.to_string())}
            }
            h1 { class: "text-title mt-2", {route.text().heading} }
            div { class: "text-body mt-3", {children} }
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
