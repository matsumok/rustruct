//! サイトの画面。Dioxus fullstack で、`web`（ブラウザ）と `server`（SSG）の feature で切り替える。
//! ネイティブの `cargo test` では feature なしで HTML を描き、ブラウザも WASM も使わない。
//!
//! 見た目はすべて役割名のクラス（`text-ink`、`text-wordmark`、`px-5` …）で書く。値は
//! `docs/design/tokens.json` にだけあり、`theme.css` を経て Tailwind のクラスになる。

use dioxus::prelude::*;

/// tokens.json → @theme の生成器。build.rs が使う。lib には test のときだけ入れて継ぎ目 B を検査する。
#[cfg(test)]
mod theme;

include!(concat!(env!("OUT_DIR"), "/fonts.rs"));

/// 上のバーに置く wordmark の文字。サイトの住所そのもの。
pub const WORDMARK: &str = "rustruct.matsumok.com";

/// dx 内蔵の Tailwind CLI が `tailwind.css` から出力した CSS。
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// サイト全体のルート。いまは上のバーの wordmark だけ。
#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        document::Stylesheet { href: GOOGLE_FONTS_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        header { class: "px-5 py-4",
            Wordmark {}
        }
    }
}

/// サイト名。`rustruct` は `ink` の 500、`.matsumok.com` は 400 で、ドットは `ink` のまま
/// `matsumok.com` だけ `muted`（tokens.json の `wordmark` の usage）。
#[component]
fn Wordmark() -> Element {
    let (name, domain) = WORDMARK
        .split_once('.')
        .expect("wordmark はドメインなのでドットを含む");
    rsx! {
        span { class: "font-mono text-wordmark text-ink",
            {name}
            span { class: "font-normal",
                "."
                span { class: "text-muted", {domain} }
            }
        }
    }
}
