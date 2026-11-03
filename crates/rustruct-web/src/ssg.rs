//! SSG（ADR site-0001）。書く側は `dx build --ssg` に「どの住所を書き出すか」と「どこへ書くか」を教え、
//! 読む側（`output`）は書き出されたものを検査する。SSG の出力の形の知識はこのファイルに置く。
//!
//! dx は server の実行ファイルを起こし、`POST /api/static_routes` が返す住所を順に GET する。
//! 描いた HTML をファイルに残すのはサーバー側の仕事で、`serve_config` がその置き場を指す。
//! 住所の列挙はルーターの enum から導き、別に一覧を持たない。

use dioxus::prelude::*;

use crate::Route;

/// not-found の静的ページの置き場。SSG はここに 1 ファイル書き、配信側が未知の住所すべてにこれを返す。
/// どの住所で開かれても同じ中身が届くので、not-found の初回描画は求められた住所を含まない。
pub const NOT_FOUND_PATH: &str = "/404";

/// SSG が書き出す住所。ルーターの静的な住所（引数のあるものは除く）と、not-found の置き場。
pub fn static_paths() -> Vec<String> {
    Route::static_routes()
        .iter()
        .map(ToString::to_string)
        .chain([NOT_FOUND_PATH.to_string()])
        .collect()
}

/// `dx build --ssg` が `POST /api/static_routes` で呼ぶ。返した住所を dx が順に取得する。
/// （`Route::static_routes()` とは別物。こちらは not-found の置き場も含む）
#[server(endpoint = "static_routes")]
pub async fn ssg_static_routes() -> ServerFnResult<Vec<String>> {
    Ok(static_paths())
}

/// `dx build --ssg` のためのサーバー設定。描いたページを `public/<住所>/index.html` に書く。
///
/// `public/` は dx が server の実行ファイルの隣に置く配信物の置き場（index.html の殻、assets/）で、
/// dioxus-server が殻を探す場所と同じ。`dx serve`（開発サーバーの下）では書かない。書くと再ビルドの
/// たびに古い HTML が残り、`/` の分が殻の index.html を上書きして次の起動で殻として読まれるため。
#[cfg(feature = "server")]
pub fn serve_config() -> dioxus::server::ServeConfig {
    use dioxus::server::{IncrementalRendererConfig, ServeConfig};

    let config = ServeConfig::new();
    if dioxus::cli_config::devserver_raw_addr().is_some() {
        return config;
    }
    let public = std::env::var_os("DIOXUS_PUBLIC_PATH")
        .map(std::path::PathBuf::from)
        .or_else(|| Some(std::env::current_exe().ok()?.parent()?.join("public")))
        .expect("server の実行ファイルの場所が分からない");
    config.incremental(IncrementalRendererConfig::new().static_dir(public))
}

/// 読む側。SSG の出力（`dx build --ssg --release --force-sequential` が出す `public/`）を外から見る。
/// 見るのは、住所ごとの HTML が静的なファイルとしてあり（SPA のフォールバックに頼らない）、`<title>` がその
/// ルートのもので、CSS と WASM が 1 つずつあり、HTML → CSS と HTML → 読込スクリプト → WASM の参照が揃うこと。
/// 本文の文言は `render_path` 越しに tests/render.rs が見るので、ここは見ない。hydration はブラウザが要るので見ない。
/// HTML と読込スクリプトを読むところは純粋関数（文字列 → 住所）で、フォルダを歩くのは `check_ssg_output`。
#[cfg(not(target_arch = "wasm32"))]
mod output {
    use std::path::{Path, PathBuf};
    use std::str::FromStr;

    use super::static_paths;
    use crate::Route;

    /// HTML の `<title>` の中身。
    pub fn page_title(html: &str) -> Option<&str> {
        let start = html.find("<title>")? + "<title>".len();
        let len = html[start..].find("</title>")?;
        Some(&html[start..start + len])
    }

    /// HTML が読むサイトの CSS の住所（`<link rel="stylesheet" href="/…">`）。外のフォントの CSS は除く。
    pub fn stylesheet(html: &str) -> Option<&str> {
        tags(html, "link")
            .filter(|tag| attr(tag, "rel") == Some("stylesheet"))
            .filter_map(|tag| attr(tag, "href"))
            .find(|href| is_site_address(href))
    }

    /// HTML が WASM を読ませるスクリプトの住所。dx は本文の後に `<script type="module" async src="…">` を置く
    /// （同じ住所の preload の link や、別の素の script も並ぶ）。
    pub fn loader_script(html: &str) -> Option<&str> {
        tags(html, "script")
            .filter(|tag| attr(tag, "type") == Some("module"))
            .filter_map(|tag| attr(tag, "src"))
            .find(|src| is_site_address(src))
    }

    /// 読込スクリプトが取りに行く WASM の住所（`"/./assets/<名前>-<ハッシュ>.wasm"` の文字列）。
    /// 住所でない素の名前（`new URL("<名前>.wasm", import.meta.url)`）も現れるが、それは除く。
    pub fn loader_wasm(js: &str) -> Option<&str> {
        js.match_indices(".wasm\"")
            .filter_map(|(at, _)| {
                let end = at + ".wasm".len();
                let start = js[..end].rfind('"')? + 1;
                Some(&js[start..end])
            })
            .find(|address| is_site_address(address))
    }

    /// `<name …>` の開きタグを順に。
    fn tags<'a>(html: &'a str, name: &str) -> impl Iterator<Item = &'a str> {
        let open = format!("<{name} ");
        html.match_indices(&open)
            .map(|(at, _)| at)
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(move |at| html[at..].find('>').map(|len| &html[at..at + len]))
    }

    /// 開きタグの `name="…"` の値。
    fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
        let key = format!(" {name}=\"");
        let start = tag.find(&key)? + key.len();
        let len = tag[start..].find('"')?;
        Some(&tag[start..start + len])
    }

    /// このサイトの住所（`/` で始まり、`//` で他のホストを指さない）。
    fn is_site_address(address: &str) -> bool {
        address.starts_with('/') && !address.starts_with("//")
    }

    /// 住所が指す出力の中のファイル。dx は `/./assets/…` の形でも指す。
    fn file_of(out: &Path, address: &str) -> PathBuf {
        out.join(address.trim_start_matches('/').trim_start_matches("./"))
    }

    /// SSG の出力 `out` を検査し、食い違いを 1 つ 1 行で返す。空なら通る。
    pub fn check_ssg_output(out: &Path) -> Vec<String> {
        if !out.is_dir() {
            return vec![format!(
                "{} が無い（先に dx build --ssg --release --force-sequential）",
                out.display()
            )];
        }
        let mut problems = Vec::new();
        let files = files_under(out);

        let mut expected = static_paths();
        expected.sort();
        let mut found: Vec<String> = files
            .iter()
            .filter(|f| f.file_name().is_some_and(|n| n == "index.html"))
            .filter_map(|f| f.parent()?.strip_prefix(out).ok())
            .map(|dir| format!("/{}", dir.to_string_lossy()))
            .collect();
        found.sort();
        if found != expected {
            problems.push(format!(
                "住所ごとの HTML が期待と違う。期待: {} / 実際: {}",
                expected.join(" "),
                found.join(" ")
            ));
        }

        // CSS と WASM は 1 つずつ。手元で dx build を重ねると前のビルドのアセットが残って 2 つになる
        // （check.sh の build は先に消す）。1 つでなければ参照の検査は飛ばす（同じ原因で NG を重ねない）
        let mut one = |label: &str, extension: &str| {
            let hits: Vec<&PathBuf> = files
                .iter()
                .filter(|f| f.extension().is_some_and(|e| e == extension))
                .collect();
            match hits.as_slice() {
                [only] => Some((*only).clone()),
                _ => {
                    let names: Vec<String> = hits.iter().map(|f| f.display().to_string()).collect();
                    problems.push(format!("{label} が 1 つでない: {}", names.join(" ")));
                    None
                }
            }
        };
        let css = one("CSS", "css");
        let wasm = one("WASM", "wasm");

        for path in &expected {
            let Ok(html) =
                std::fs::read_to_string(out.join(path.trim_start_matches('/')).join("index.html"))
            else {
                continue; // 住所の集合の検査で既に NG
            };
            if let Ok(route) = Route::from_str(path) {
                let want = route.title();
                if page_title(&html) != Some(want.as_str()) {
                    problems.push(format!(
                        "{path} の <title> が「{want}」でない: {}",
                        page_title(&html).unwrap_or("（無し）")
                    ));
                }
            }
            if let Some(css) = &css
                && stylesheet(&html).map(|href| file_of(out, href)) != Some(css.clone())
            {
                problems.push(format!("{path} が CSS {} を参照していない", css.display()));
            }
            let Some(js) = loader_script(&html) else {
                problems.push(format!("{path} の HTML に WASM の読込スクリプトが無い"));
                continue;
            };
            let Ok(loader) = std::fs::read_to_string(file_of(out, js)) else {
                problems.push(format!("{path} が参照するスクリプトが無い: {js}"));
                continue;
            };
            if let Some(wasm) = &wasm
                && loader_wasm(&loader).map(|address| file_of(out, address)) != Some(wasm.clone())
            {
                problems.push(format!(
                    "{path} のスクリプト {js} が WASM {} を参照していない",
                    wasm.display()
                ));
            }
        }
        problems
    }

    /// `dir` の下のファイルすべて（深さ優先、名前順）。
    fn files_under(dir: &Path) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut paths: Vec<PathBuf> = entries.filter_map(|e| Some(e.ok()?.path())).collect();
        paths.sort();
        paths
            .into_iter()
            .flat_map(|p| if p.is_dir() { files_under(&p) } else { vec![p] })
            .collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use output::{check_ssg_output, loader_script, loader_wasm, page_title, stylesheet};
