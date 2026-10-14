//! 継ぎ目 A: パスを受け取り、そのページの HTML 文字列を返す。
//!
//! ブラウザが描くのと同じ `App` を、メモリ上の履歴（`MemoryHistory`）でそのパスに置いて描く。
//! `document::Title` や `document::Meta` が head に入れようとするものは `HeadCollector` が受け取り、
//! `<head>` に書く。ネイティブの `cargo test` がここを通して各ルートの文字と `<title>` を検査する。
//!
//! `<html>` の殻は検査のための最小のもので、`dx build --ssg` が出す殻（WASM の読込を含む）とは別。
//! App が head と body に出すものだけがこの関数の見るところ。

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;

use dioxus::document::{Document, Eval, NoOpDocument};
use dioxus::history::MemoryHistory;
use dioxus::prelude::*;

use crate::App;

/// `path`（`/tools` など）を開いたときのページ全体の HTML。
pub fn render_path(path: &str) -> String {
    let head = Rc::new(HeadCollector::default());
    let mut dom = VirtualDom::new(App);
    dom.provide_root_context(Rc::new(MemoryHistory::with_initial_path(path)) as Rc<dyn History>);
    dom.provide_root_context(head.clone() as Rc<dyn Document>);
    dom.rebuild_in_place();

    let body = dioxus_ssr::render(&dom);
    let head_html = head.0.borrow();
    format!(
        "<!DOCTYPE html>\n<html lang=\"ja\"><head><meta charset=\"utf-8\">{head_html}</head><body>{body}</body></html>"
    )
}

/// head に入れるものを描画の間に HTML として集める `Document`。JS は動かさない。
#[derive(Default)]
struct HeadCollector(RefCell<String>);

impl Document for HeadCollector {
    fn eval(&self, js: String) -> Eval {
        NoOpDocument.eval(js)
    }

    fn set_title(&self, title: String) {
        let mut head = self.0.borrow_mut();
        write!(head, "<title>{}</title>", escape(&title)).expect("String への書込");
    }

    // meta / link / script / style はすべて既定実装からここに来る
    fn create_head_element(
        &self,
        name: &str,
        attributes: &[(&str, String)],
        contents: Option<String>,
    ) {
        let mut head = self.0.borrow_mut();
        write!(head, "<{name}").expect("String への書込");
        for (key, value) in attributes {
            write!(head, " {key}=\"{}\"", escape(value)).expect("String への書込");
        }
        head.push('>');
        if !matches!(name, "meta" | "link") {
            // script / style の中身は HTML ではないのでそのまま
            head.push_str(contents.as_deref().unwrap_or_default());
            write!(head, "</{name}>").expect("String への書込");
        }
    }
}

/// 文字と属性値のエスケープ。
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
