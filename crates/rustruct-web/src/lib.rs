//! サイトの画面。Dioxus fullstack で、`web`（ブラウザ）と `server`（SSG）の feature で切り替える。
//! ネイティブの `cargo test` では feature なしで HTML を描き、ブラウザも WASM も使わない。
//!
//! 見た目はすべて役割名のクラス（`text-ink`、`text-wordmark`、`gap-4` …）で書く。値は
//! `docs/design/tokens.json` にだけあり、`theme.css` を経て Tailwind のクラスになる。

use dioxus::prelude::*;

/// tokens.json → @theme の生成器。build.rs が使う。lib には test のときだけ入れて継ぎ目 B を検査する。
#[cfg(test)]
mod theme;

/// 開きタグを属性で数えるテストの道具。結合テストの `common` と同じもの
#[cfg(test)]
#[path = "../tests/common/markup.rs"]
mod markup;

/// 継ぎ目 A: パス → HTML 文字列。ネイティブだけ（ブラウザは hydration で同じ App を描く）。
#[cfg(not(target_arch = "wasm32"))]
mod prerender;
#[cfg(not(target_arch = "wasm32"))]
pub use prerender::render_path;

/// SSG: 書き出す住所の列挙と、書く場所の設定。書き出したものを読む検査。
mod ssg;

/// 計算のページ（Data と Tool）の表示用の表と列の部品。
mod table;

/// Tool の入力の Sheet の部品。
mod sheet;

/// 入力の組: 欄の定義、値の置き場、Sheet と注記・理由・警告の部品。
mod inputs;

/// Data: 材料や規格の値のページ。
mod data;

/// Tools: 一覧（分野の見出しごと）と、各 Tool のページ。
mod tools;

/// 図の色と書体（charta の Theme）と、図の SVG。
mod chart;

/// Tool「Kirchhoff 板の曲げ」のページ。
mod kirchhoff_plate;

/// Tool「必要スラブ厚さ」のページ。
mod slab_thickness;

/// ヘッダーのボタンのアイコン（Carbon）。
mod icons;

/// サイトの住所の木: ルーター、ルートごとの文言、載る一覧、パンくずとメニューの段。
mod route;

/// シェル: ヘッダー（サイト名・パンくず・ライトとダーク・アカウント）とメニュー。
mod shell;

/// ページの中身: トップ、ページの一覧、下層ページの枠、まだ中身の無いページ。
mod page;

pub use data::ConcreteReadout;
pub use kirchhoff_plate::{
    InputField, PlateFigure, PlateInputs, PlateOutcome, PlateReadout, Quantity,
};
pub use route::{PageText, Route, WORDMARK};
pub use slab_thickness::{SlabField, SlabFigure, SlabInputs, SlabOutcome, SlabReadout};
#[cfg(feature = "server")]
pub use ssg::serve_config;
pub use ssg::{NOT_FOUND_PATH, static_paths};
#[cfg(not(target_arch = "wasm32"))]
pub use ssg::{check_ssg_output, loader_script, loader_wasm, page_title, stylesheet};

include!(concat!(env!("OUT_DIR"), "/fonts.rs"));

/// dx 内蔵の Tailwind CLI が `tailwind.css` から出力した CSS。
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// サイト全体のルート。フォントと CSS の読込と、ルーター。
#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        document::Stylesheet { href: GOOGLE_FONTS_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        document::Script { {shell::APPLY_REMEMBERED_THEME} }
        Router::<Route> {}
    }
}
