//! `docs/design/tokens.json` から生成物を書く。
//!
//! - `theme.css`（Tailwind v4 の `@theme`）: `tailwind.css` が `@import` し、`dx` 内蔵の Tailwind CLI
//!   が読む。リポジトリに入れる生成物なので、内容が変わらないときは書き込まない（書くと dx の
//!   監視が無駄に走る）
//! - `$OUT_DIR/fonts.rs`（Google Fonts の URL と、図の座標の幅にする本文の1単位）: `lib.rs` が `include!` する
//! - `$OUT_DIR/chart_theme.rs`（図の色の役割名・ライト・ダークの表と書体）: `chart.rs` が `include!` して
//!   `charta::Theme` を作る。tokens.json を読むのはこのクレートだけ（charta は値を受け取る）
//! - `$OUT_DIR/content.rs`（`content/` の Markdown を HTML にしたものと、その住所）: `prose.rs` が `include!` する。
//!   `content/<住所>.md` が住所 `/<住所>` の本文で、式も KaTeX でここで組む（ADR site-0019）
//!
//! 変換そのものは `src/theme.rs` と `src/markdown.rs` の純粋関数で、ここはファイルの読み書きだけを持つ。

#[allow(dead_code)]
#[path = "src/theme.rs"]
mod theme;

#[path = "src/markdown.rs"]
mod markdown;

use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let tokens_path = manifest_dir.join("../../docs/design/tokens.json");
    let theme_path = manifest_dir.join("theme.css");
    let fonts_path = Path::new(&out_dir).join("fonts.rs");

    println!("cargo:rerun-if-changed={}", tokens_path.display());
    println!("cargo:rerun-if-changed=src/theme.rs");
    println!("cargo:rerun-if-changed=src/markdown.rs");
    println!("cargo:rerun-if-changed=content");
    println!("cargo:rerun-if-changed=build.rs");

    let tokens = std::fs::read_to_string(&tokens_path)
        .unwrap_or_else(|e| panic!("{} を読めない: {e}", tokens_path.display()));

    let css = theme::theme_css(&tokens).unwrap_or_else(|e| panic!("{e}"));
    if std::fs::read_to_string(&theme_path).ok().as_deref() != Some(css.as_str()) {
        std::fs::write(&theme_path, css)
            .unwrap_or_else(|e| panic!("{} を書けない: {e}", theme_path.display()));
    }

    let url = theme::google_fonts_url(&tokens).unwrap_or_else(|e| panic!("{e}"));
    let column = theme::column_width(&tokens).unwrap_or_else(|e| panic!("{e}"));
    let fonts_rs = format!(
        "/// Google Fonts の CSS。tokens.json の families と文字スタイルの太さから build.rs が導いた。\n\
         const GOOGLE_FONTS_CSS: &str = {url:?};\n\
         /// 本文の1単位（tokens.json の `width-column`）。図の座標の幅。\n\
         pub(crate) const COLUMN_WIDTH: f64 = {column}.0;\n"
    );
    std::fs::write(&fonts_path, fonts_rs)
        .unwrap_or_else(|e| panic!("{} を書けない: {e}", fonts_path.display()));

    let chart_path = Path::new(&out_dir).join("chart_theme.rs");
    let chart_rs = theme::chart_theme_rs(&tokens).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&chart_path, chart_rs)
        .unwrap_or_else(|e| panic!("{} を書けない: {e}", chart_path.display()));

    let content_dir = manifest_dir.join("content");
    let mut pages = Vec::new();
    markdown_files(&content_dir, &mut pages);
    pages.sort();
    let mut content_rs = String::from(
        "/// `content/` の Markdown を HTML にしたもの（build.rs が書く）。住所と本文の HTML の組で、住所の順。\n\
         const CONTENT: &[(&str, &str)] = &[\n",
    );
    for path in pages {
        let relative = path
            .strip_prefix(&content_dir)
            .expect("content の下")
            .with_extension("");
        let address = format!("/{}", relative.to_string_lossy());
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} を読めない: {e}", path.display()));
        let html = markdown::to_html(&source).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        content_rs.push_str(&format!("    ({address:?}, {html:?}),\n"));
    }
    content_rs.push_str("];\n");
    let content_path = Path::new(&out_dir).join("content.rs");
    std::fs::write(&content_path, content_rs)
        .unwrap_or_else(|e| panic!("{} を書けない: {e}", content_path.display()));
}

/// `dir` の下の `.md` のファイルすべて
fn markdown_files(dir: &Path, found: &mut Vec<std::path::PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{} を読めない: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("content の項目").path();
        if path.is_dir() {
            markdown_files(&path, found);
        } else if path.extension().is_some_and(|e| e == "md") {
            found.push(path);
        }
    }
}
