//! サイトの画面。Dioxus fullstack で、`web`（ブラウザ）と `server`（SSG）の feature で切り替える。
//! ネイティブの `cargo test` では feature なしで HTML を描き、ブラウザも WASM も使わない。

use dioxus::prelude::*;

/// 上のバーに置く wordmark の文字。サイトの住所そのもの。
pub const WORDMARK: &str = "rustruct.matsumok.com";

/// サイト全体のルート。いまは wordmark の文字だけで、スタイルも住所も持たない。
#[component]
pub fn App() -> Element {
    rsx! {
        header { {WORDMARK} }
    }
}
