//! Issue #8 の継ぎ目 A（パス → HTML 文字列）の前段。まだパスは無いので、App を HTML に描いた結果だけを見る。

use dioxus::prelude::*;

#[test]
fn app_renders_wordmark_text() {
    let html = dioxus_ssr::render_element(rsx! { rustruct_web::App {} });
    assert!(
        html.contains("rustruct.matsumok.com"),
        "wordmark が HTML に無い: {html}"
    );
}
