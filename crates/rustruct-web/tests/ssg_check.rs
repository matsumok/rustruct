//! SSG の出力を読む側（`src/ssg.rs`、example `check-ssg`、Issue #12・#94）。
//! HTML と読込スクリプトを読む純粋関数には dx の出力の短い例を渡す。出力のフォルダを歩く `check_ssg_output` には、
//! `static_paths` の住所ごとに `render_path` の HTML を置き、dx が足す CSS・読込スクリプト・WASM を加えた見本を渡す。

mod common;
use std::path::{Path, PathBuf};

use common::{temp_dir, write};
use rustruct_web::{
    check_ssg_output, loader_script, loader_wasm, page_title, render_path, static_paths, stylesheet,
};

/// dx の出力の `<head>` の短い例（2026-11-03 の `dx build --ssg --release` から、属性の形をそのまま残して縮めた）。
/// 読込スクリプトの preload と、別の素の script（focus-trap）が並ぶ。
const HEAD: &str = concat!(
    "<head>\n<title>Tools · rustruct.matsumok.com</title>\n",
    r#"<link rel="preload" as="script" href="/./assets/rustruct-web-dxhacd2.js" crossorigin>"#,
    r#"<meta name="description" content="構造計算の入力・計算・判定。"/>"#,
    r#"<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono&#38;display=swap" type="text/css"/>"#,
    r#"<link rel="stylesheet" href="/assets/tailwind-dxh4271.css" type="text/css"/>"#,
    r#"<script src="/assets/focus-trap-dxh66a5.js" defer=true></script></head>"#,
);

/// 本文の後に dx が置く読込スクリプト。
const BODY_END: &str = concat!(
    "<div id=\"main\"></div>\n",
    r#"    <script type="module" async src="/./assets/rustruct-web-dxhacd2.js"></script>"#,
    "\n</body>",
);

/// 読込スクリプト（wasm-bindgen の出力を dx が縮めたもの）の短い例。素の名前と住所の 2 つで WASM が現れる。
const LOADER: &str = concat!(
    r#"s===void 0&&(s=new URL("rustruct-web_bg.wasm",import.meta.url));"#,
    r#"gt({module_or_path:"/./assets/rustruct-web_bg-dxhe2c5.wasm"}).then(s=>{globalThis.x=s})"#,
);

#[test]
fn the_page_title_is_read_from_the_head() {
    assert_eq!(page_title(HEAD), Some("Tools · rustruct.matsumok.com"));
    assert_eq!(page_title("<head></head>"), None);
}

#[test]
fn the_site_stylesheet_is_read_and_the_font_stylesheet_is_skipped() {
    assert_eq!(stylesheet(HEAD), Some("/assets/tailwind-dxh4271.css"));
    assert_eq!(stylesheet(BODY_END), None);
}

#[test]
fn the_loader_is_the_module_script_not_the_preload_or_the_plain_script() {
    let html = format!("{HEAD}{BODY_END}");
    assert_eq!(
        loader_script(&html),
        Some("/./assets/rustruct-web-dxhacd2.js")
    );
    assert_eq!(loader_script(HEAD), None);
}

#[test]
fn the_loader_names_the_wasm_by_address_not_by_its_bare_name() {
    assert_eq!(
        loader_wasm(LOADER),
        Some("/./assets/rustruct-web_bg-dxhe2c5.wasm")
    );
    assert_eq!(
        loader_wasm(r#"new URL("rustruct-web_bg.wasm",import.meta.url)"#),
        None
    );
}

/// SSG の出力の形をした見本。CSS・読込スクリプト・WASM が 1 つずつで、参照が揃っている。
/// `render_path` は CSS をソースのファイルの場所で指すので、dx が配る住所に置き換える。
fn ssg_fixture(root: &Path) -> PathBuf {
    let out = root.join("public");
    let source_css = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/tailwind.css");
    let body = concat!(
        r#"<script type="module" async src="/./assets/rustruct-web-dxh1.js"></script>"#,
        "</body>",
    );
    for path in static_paths() {
        let html = render_path(&path)
            .replacen(source_css, "/assets/tailwind-dxh1.css", 1)
            .replacen("</body>", body, 1);
        write(
            out.join(path.trim_start_matches('/')).join("index.html"),
            &html,
        );
    }
    write(out.join("assets/tailwind-dxh1.css"), "");
    write(
        out.join("assets/rustruct-web-dxh1.js"),
        r#"gt({module_or_path:"/./assets/rustruct-web_bg-dxh1.wasm"})"#,
    );
    write(out.join("assets/rustruct-web_bg-dxh1.wasm"), "");
    out
}

#[test]
fn a_complete_ssg_output_passes() {
    let out = ssg_fixture(&temp_dir("ssg-ok"));
    assert_eq!(check_ssg_output(&out), Vec::<String>::new());
}

#[test]
fn a_missing_output_folder_says_to_build_first() {
    let problems = check_ssg_output(&temp_dir("ssg-none").join("public"));
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("dx build --ssg"), "{problems:?}");
}

#[test]
fn a_page_missing_from_the_output_is_named() {
    let out = ssg_fixture(&temp_dir("ssg-missing"));
    std::fs::remove_dir_all(out.join("data/rebar")).unwrap();
    let problems = check_ssg_output(&out);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("/data/rebar"), "{problems:?}");
}

#[test]
fn a_page_whose_title_is_not_its_routes_title_is_named() {
    let out = ssg_fixture(&temp_dir("ssg-title"));
    write(
        out.join("tools/index.html"),
        &std::fs::read_to_string(out.join("data/index.html")).unwrap(),
    );
    let problems = check_ssg_output(&out);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("/tools の <title>") && p.contains("Tools · ")),
        "{problems:?}"
    );
}

#[test]
fn a_loader_pointing_at_no_wasm_is_named() {
    let out = ssg_fixture(&temp_dir("ssg-loader"));
    write(out.join("assets/rustruct-web-dxh1.js"), "gt({})");
    let problems = check_ssg_output(&out);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("rustruct-web-dxh1.js") && p.contains("WASM")),
        "{problems:?}"
    );
}

#[test]
fn a_stale_wasm_left_by_an_earlier_build_is_named_as_such() {
    // 手元で dx build を重ねると、前のビルドのハッシュ付きアセットが public/assets に残る（2026-10-14）
    let out = ssg_fixture(&temp_dir("ssg-stale"));
    write(out.join("assets/rustruct-web_bg-dxh0.wasm"), "");
    let problems = check_ssg_output(&out);
    assert_eq!(
        problems.len(),
        1,
        "WASM が 2 つのときに参照の NG まで出ている: {problems:?}"
    );
    assert!(problems[0].contains("WASM が 1 つでない"), "{problems:?}");
}
