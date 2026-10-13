//! Issue #8 の継ぎ目 A（パス → HTML 文字列）の前段。まだパスは無いので、App を HTML に描いた結果だけを見る。
//! クラス名の並びは見ない。本文の文字と、どこで要素が分かれているかだけを見る。

use dioxus::prelude::*;

#[test]
fn app_renders_wordmark_text() {
    let html = dioxus_ssr::render_element(rsx! { rustruct_web::App {} });
    assert!(
        text_of(&html).contains("rustruct.matsumok.com"),
        "wordmark が HTML に無い: {html}"
    );
}

/// タグを外した本文。
fn text_of(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

#[test]
fn wordmark_is_split_so_the_dot_and_domain_can_be_muted() {
    let html = dioxus_ssr::render_element(rsx! { rustruct_web::App {} });
    // .matsumok.com がドットごと別の要素に入っている（muted にするため）
    let domain = html
        .find(">.matsumok.com<")
        .expect(".matsumok.com が単独の要素でない");
    let rustruct = html.find("rustruct").expect("rustruct が無い");
    assert!(rustruct < domain, "順序が違う: {html}");
}
