//! サイトの住所の木。ルーター（`Route`）と、ルートごとの文言・載る一覧・パンくずの段・メニューの枝。
//! 一覧もパンくずもメニューも、ここにあるルーターの住所一覧から導く。

use dioxus::prelude::*;

use crate::data::{ConcretePage, DataIndex, RebarPage};
use crate::kirchhoff_plate::KirchhoffPlatePage;
use crate::page::{Home, NotFound, RootPage, SectionPage};
use crate::shell::Frame;
use crate::slab_thickness::SlabThicknessPage;
use crate::tools::{Domain, ToolsIndex};

/// ヘッダーに置く wordmark の文字。サイトの住所そのもの。
pub const WORDMARK: &str = "rustruct.matsumok.com";

/// サイトの住所。全ページが `Frame`（ヘッダーと本文の枠）の中に入る。
#[derive(Routable, Clone, PartialEq, Debug)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Frame)]
        #[route("/")]
        Home {},
        #[route("/tools", ToolsIndex)]
        Tools {},
        #[route("/tools/kirchhoff-plate", KirchhoffPlatePage)]
        ToolsKirchhoffPlate {},
        #[route("/tools/slab-thickness", SlabThicknessPage)]
        ToolsSlabThickness {},
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
    /// `/tools` の一覧の、分野の見出しの下
    Tools(Domain),
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
            Route::ToolsKirchhoffPlate {} => (
                "Kirchhoff 板の曲げ",
                "長方形の板の、たわみ・曲げモーメント・反力。",
            ),
            Route::ToolsSlabThickness {} => (
                "必要スラブ厚さ",
                "四辺固定の RC スラブの、たわみと表18-1 から決まる必要厚さ。",
            ),
            Route::Data {} => ("Data", "構造設計で参照する材料と規格の値。"),
            Route::DataConcrete {} => (
                "コンクリート",
                "Fc から気乾単位体積重量・ヤング係数・許容応力度を求める。",
            ),
            Route::DataRebar {} => (
                "鉄筋",
                "異形棒鋼の呼び名ごとの寸法と単位質量、種類ごとの許容応力度。",
            ),
            Route::Studies {} => ("Studies", "数学やかたちを眺めて考えるための可視化。"),
            Route::Notes {} => ("Notes", "構造や数学についての書きもの。"),
            Route::Root {} => ("root", "root のための場所。"),
            Route::NotFound { .. } => ("Not found", "ページが見つからない。"),
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
            Route::ToolsKirchhoffPlate {} => Listing::Tools(Domain::Mechanics),
            Route::ToolsSlabThickness {} => Listing::Tools(Domain::Rc),
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

    /// `/tools` の分野 `domain` の見出しの下に並べる Tool。四部と同じく、ルーターの住所一覧から導く。
    pub(crate) fn tools_in(domain: Domain) -> impl Iterator<Item = Route> {
        Route::listed_on(Listing::Tools(domain))
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
    /// どの住所でも同じ 1 ファイルなので、住所を出さない（ADR site-0001 の規則）。
    pub(crate) fn trail(&self) -> Vec<(String, Option<Route>)> {
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
    pub(crate) fn children(&self) -> Vec<Route> {
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

    /// `other` がこのページか、その下のページか。not-found はどこにも入らない（ADR site-0001 の規則）。
    pub(crate) fn holds(&self, other: &Route) -> bool {
        if matches!(other, Route::NotFound { .. }) {
            return false;
        }
        let (base, path) = (self.to_string(), other.to_string());
        path == base || path.starts_with(&format!("{base}/"))
    }
}
