//! `docs/design/tokens.json` から生成物を書く。
//!
//! - `theme.css`（Tailwind v4 の `@theme`）: `tailwind.css` が `@import` し、`dx` 内蔵の Tailwind CLI
//!   が読む。リポジトリに入れる生成物なので、内容が変わらないときは書き込まない（書くと dx の
//!   監視が無駄に走る）
//! - `$OUT_DIR/fonts.rs`（Google Fonts の URL と、図の座標の幅にする本文の1単位）: `lib.rs` が `include!` する
//!
//! 変換そのものは `src/theme.rs` の純粋関数で、ここはファイルの読み書きだけを持つ。

#[allow(dead_code)]
#[path = "src/theme.rs"]
mod theme;

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
}
