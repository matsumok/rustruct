//! 配置後のスモークテスト（`scripts/check-live.sh`、Issue #13・#21・#24）そのものを確かめる。
//! 手元に配置先の形をした偽サーバーを立て、スクリプトをそこへ向ける。サーバーとスクリプトはどちらもサンドボックスの中で動く。
//! ページは `static_paths` の住所ごとの `render_path`。`/assets/` は拡張子で中身を返す。どちらも最初の何回かを 404 にできる
//! （配置で上書きした直後に古い版が返り、新しいアセットやページが届かない、#21・#24 の再現）。

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use rustruct_web::{NOT_FOUND_PATH, render_path, static_paths};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// HTML に dx が足すもの: CSS と WASM の読込スクリプト
const HEAD: &str = concat!(
    "\n<link rel=\"stylesheet\" href=\"/assets/tailwind-dxh1.css\">\n",
    "<script type=\"module\" async src=\"/./assets/rustruct-web-dxh1.js\"></script>\n</head>",
);

struct Response {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: Vec<u8>,
}

fn response(status: u16, content_type: &str, body: impl Into<Vec<u8>>) -> Response {
    Response {
        status,
        headers: vec![("Content-Type", content_type.to_string())],
        body: body.into(),
    }
}

/// 配置先の振る舞い（`wrangler.toml` の `auto-trailing-slash` と `404-page`）。
/// ページの住所はそれぞれ最初の `page_404s` 回だけ not-found の本文で 404 を、`/assets/` の住所はそれぞれ最初の
/// `asset_404s` 回だけ 404 を返す。
fn serve(page_404s: usize, asset_404s: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("偽サーバーの port が取れない");
    let addr = listener.local_addr().unwrap();
    let pages: HashMap<String, String> = static_paths()
        .into_iter()
        .filter(|p| p != NOT_FOUND_PATH)
        .map(|p| {
            let html = render_path(&p).replacen("</head>", HEAD, 1);
            (if p == "/" { p } else { format!("{p}/") }, html)
        })
        .collect();
    let not_found = render_path(NOT_FOUND_PATH);
    let hits = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut line = String::new();
            let mut reader = BufReader::new(&stream);
            reader.read_line(&mut line).unwrap();
            // 残りのヘッダは読み捨てる
            let mut header = String::new();
            while reader.read_line(&mut header).unwrap() > 2 {
                header.clear();
            }
            let path = line
                .split(' ')
                .nth(1)
                .unwrap_or("/")
                .replacen("/./", "/", 1);
            let n = {
                let mut hits = hits.lock().unwrap();
                let n = hits.entry(path.clone()).or_default();
                *n += 1;
                *n
            };
            let r = if let Some(asset) = path.strip_prefix("/assets/") {
                if n <= asset_404s {
                    response(404, "text/plain", "")
                } else if asset.ends_with(".css") {
                    response(200, "text/css", "@media (prefers-color-scheme: dark) {}")
                } else if asset.ends_with(".js") {
                    response(
                        200,
                        "text/javascript",
                        r#"fetch("/./assets/rustruct-web_bg-dxh1.wasm")"#,
                    )
                } else if asset.ends_with(".wasm") {
                    response(200, "application/wasm", "\0asm")
                } else {
                    response(404, "text/plain", "")
                }
            } else if let Some(html) = pages.get(&path) {
                if n <= page_404s {
                    response(404, "text/html", not_found.as_str())
                } else {
                    response(200, "text/html", html.as_str())
                }
            } else if pages.contains_key(&format!("{path}/")) {
                Response {
                    status: 307,
                    headers: vec![("Location", format!("{path}/"))],
                    body: vec![],
                }
            } else {
                response(404, "text/html", not_found.as_str())
            };
            let mut head = format!(
                "HTTP/1.1 {} X\r\nConnection: close\r\nContent-Length: {}\r\n",
                r.status,
                r.body.len()
            );
            for (name, value) in r.headers {
                head.push_str(&format!("{name}: {value}\r\n"));
            }
            head.push_str("\r\n");
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&r.body);
        }
    });
    format!("http://{addr}")
}

fn check_live(url: &str) -> (Output, String) {
    let tmp =
        std::env::temp_dir().join(format!("rustruct-live-{}", url.rsplit(':').next().unwrap()));
    std::fs::create_dir_all(&tmp).unwrap();
    let output = Command::new("sh")
        .arg(workspace_root().join("scripts/check-live.sh"))
        .arg(url)
        .env("CHECK_LIVE_RETRY_SECONDS", "1")
        // 取得物の置き場をテストごとに分ける（並列に走っても混ざらない）
        .env("TMPDIR", &tmp)
        .env_remove("CF_ACCESS_CLIENT_ID")
        .env_remove("CF_ACCESS_CLIENT_SECRET")
        .output()
        .expect("sh が起動できない");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output, stdout)
}

#[test]
fn a_site_serving_every_page_and_asset_passes_without_waiting() {
    let (output, stdout) = check_live(&serve(0, 0));
    assert!(output.status.success(), "{stdout}");
    assert!(!stdout.contains("待"), "待たなくてよいのに待った: {stdout}");
}

#[test]
fn assets_that_404_just_after_a_deploy_are_fetched_again() {
    let (output, stdout) = check_live(&serve(0, 1));
    assert!(output.status.success(), "{stdout}");
    for asset in [
        "/assets/tailwind-dxh1.css は 1 秒待った",
        "/./assets/rustruct-web-dxh1.js は 1 秒待った",
        "/assets/rustruct-web_bg-dxh1.wasm は 1 秒待った",
    ] {
        assert!(stdout.contains(asset), "「{asset}」が無い: {stdout}");
    }
}

#[test]
fn assets_that_keep_404ing_fail_once_the_retries_run_out() {
    let (output, stdout) = check_live(&serve(0, usize::MAX));
    assert!(!output.status.success(), "{stdout}");
    assert!(
        stdout.contains("/assets/tailwind-dxh1.css は 12 秒待った"),
        "{stdout}"
    );
    assert!(
        stdout.contains("NG: CSS /assets/tailwind-dxh1.css が 200 でない: 404"),
        "{stdout}"
    );
    // 持ち時間はページとアセットで共有する。CSS で使い切ったら、読込スクリプトは待たずに判定する
    assert!(
        !stdout.contains("rustruct-web-dxh1.js は"),
        "持ち時間を使い切ったのにまた待った: {stdout}"
    );
}

#[test]
fn pages_that_404_just_after_a_deploy_are_fetched_again() {
    let (output, stdout) = check_live(&serve(1, 0));
    assert!(output.status.success(), "{stdout}");
    // / は最初の接続の確かめで 1 回目を使うので、待つのはそれ以外のページ
    for page in [
        "/data/ は 1 秒待った",
        "/data/concrete/ は 1 秒待った",
        "/data/rebar/ は 1 秒待った",
    ] {
        assert!(stdout.contains(page), "「{page}」が無い: {stdout}");
    }
}

#[test]
fn pages_that_keep_404ing_fail_once_the_retries_run_out() {
    let (output, stdout) = check_live(&serve(usize::MAX, 0));
    assert!(!output.status.success(), "{stdout}");
    assert!(stdout.contains("/ は 12 秒待った"), "{stdout}");
    assert!(stdout.contains("NG: / が 200 でない: 404"), "{stdout}");
    assert!(stdout.contains("NG: /data/ が 200 でない: 404"), "{stdout}");
    assert!(
        !stdout.contains("/tools/ は"),
        "持ち時間を使い切ったのにまた待った: {stdout}"
    );
}
