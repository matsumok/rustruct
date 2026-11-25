//! 説明書（`GLOSSARY.md`）のページ。本文は `content/<住所>.md` の Markdown で、`build.rs` がビルドのときに HTML にする
//! （`markdown.rs`、ADR site-0019）。ページは住所から本文を引いて置くだけで、ブラウザでは Markdown も KaTeX も動かさない。
//! SSG の HTML と hydration の WASM が同じ文字列を持つので、描き分けは起きない。

use dioxus::prelude::*;

use crate::Route;
use crate::page::Page;

include!(concat!(env!("OUT_DIR"), "/content.rs"));

/// 住所 `address` の本文の HTML。`content/` に無ければ `None`
pub(crate) fn content(address: &str) -> Option<&'static str> {
    CONTENT
        .iter()
        .find(|(at, _)| *at == address)
        .map(|(_, html)| *html)
}

/// `content/` に本文のある住所
#[cfg(test)]
pub(crate) fn addresses() -> impl Iterator<Item = &'static str> {
    CONTENT.iter().map(|(at, _)| *at)
}

/// 説明書のページ。見出しと、住所の本文。本文の文字は `prose` で、節と段落の並びは `markdown.rs` が組む。
/// 節の間は `space-7`（`docs/design/README.md`「説明書の本文」）
#[component]
pub(crate) fn ProsePage() -> Element {
    let route = use_route::<Route>();
    let html =
        content(&route.to_string()).unwrap_or_else(|| panic!("{route} の本文が content/ に無い"));
    rsx! {
        Page { route,
            article { class: "flex flex-col gap-7 text-prose", dangerous_inner_html: html }
        }
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::Routable;

    use super::addresses;
    use crate::Route;

    /// `content/` の本文はどれもルーターの住所にある（置いたのに届かない本文が無い）
    #[test]
    fn every_content_file_is_at_a_route() {
        let routes: Vec<String> = Route::static_routes()
            .iter()
            .map(ToString::to_string)
            .collect();
        for address in addresses() {
            assert!(
                routes.iter().any(|r| r == address),
                "content/{address}.md の住所がルーターに無い"
            );
        }
    }
}
