//! SSG（ADR-0001）。`dx build --ssg` に「どの住所を書き出すか」と「どこへ書くか」を教える。
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
