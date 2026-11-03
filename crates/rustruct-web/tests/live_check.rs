//! 配置後のスモークテスト（`scripts/check-live.sh`、Issue #13・#21・#24・#94）そのものを確かめる。
//! 手元に配置先の形をした偽サーバーを立て、スクリプトをそこへ向ける。サーバーとスクリプトはどちらもサンドボックスの中で動く。
//! 偽サーバーは配信用ディレクトリをそのまま配り、配置先の転送と 404 をまね、どのファイルも最初の何回かを 404 にできる
//! （配置で上書きした直後に古い版が返り、新しいアセットやページが届かない、#21・#24 の再現）。

mod common;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{temp_dir, workspace_root, write};

/// 配信用ディレクトリ（`scripts/stage-deploy.sh` の出力）の形をした見本。
fn deploy_fixture(root: &Path) -> PathBuf {
    let deploy = root.join("deploy");
    for (file, title) in [
        ("index.html", "Home · site"),
        ("tools/index.html", "Tools · site"),
        ("data/index.html", "Data · site"),
        ("data/concrete/index.html", "コンクリート · site"),
        ("404.html", "Not found · site"),
    ] {
        write(
            deploy.join(file),
            &format!("<html><head><title>{title}</title></head><body></body></html>"),
        );
    }
    write(deploy.join("assets/tailwind-dxh1.css"), "");
    write(deploy.join("assets/rustruct-web-dxh1.js"), "");
    write(deploy.join("assets/rustruct-web_bg-dxh1.wasm"), "\0asm");
    deploy
}

/// 配置先の振る舞い（`wrangler.toml` の `auto-trailing-slash` と `404-page`）で `dir` を配る。
/// `/x/` は `x/index.html`、`/x` は `x/index.html` があれば `/x/` へ 307、ファイルはそのまま、無ければ 404.html を 404 で。
/// ファイルのある住所は、それぞれ最初の `first_404s` 回だけ 404 を返す。
fn serve(dir: PathBuf, first_404s: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("偽サーバーの port が取れない");
    let addr = listener.local_addr().unwrap();
    let mut hits = HashMap::<String, usize>::new();
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
            let path = line.split(' ').nth(1).unwrap_or("/").to_string();
            let n = hits.entry(path.clone()).or_default();
            *n += 1;
            let rel = path.trim_start_matches('/');
            let file = if path.ends_with('/') {
                dir.join(rel).join("index.html")
            } else {
                dir.join(rel)
            };
            let not_found = || std::fs::read(dir.join("404.html")).unwrap();
            let (status, headers, body) = if file.is_file() && *n <= first_404s {
                (404, vec![], not_found())
            } else if file.is_file() {
                let content_type = match file.extension().and_then(|e| e.to_str()) {
                    Some("html") => "text/html",
                    Some("css") => "text/css",
                    Some("js") => "text/javascript",
                    Some("wasm") => "application/wasm",
                    _ => "application/octet-stream",
                };
                let body = std::fs::read(&file).unwrap();
                (200, vec![format!("Content-Type: {content_type}")], body)
            } else if dir.join(rel).join("index.html").is_file() {
                (307, vec![format!("Location: {path}/")], vec![])
            } else {
                (404, vec![], not_found())
            };
            let mut head = format!(
                "HTTP/1.1 {status} X\r\nConnection: close\r\nContent-Length: {}\r\n",
                body.len()
            );
            for h in headers {
                head.push_str(&format!("{h}\r\n"));
            }
            head.push_str("\r\n");
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    format!("http://{addr}")
}

fn check_live(url: &str, deploy: &Path) -> (Output, String) {
    let tmp =
        std::env::temp_dir().join(format!("rustruct-live-{}", url.rsplit(':').next().unwrap()));
    std::fs::create_dir_all(&tmp).unwrap();
    let output = Command::new("sh")
        .arg(workspace_root().join("scripts/check-live.sh"))
        .arg(url)
        .arg(deploy)
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
fn a_site_serving_the_deploy_dir_passes_without_waiting() {
    let deploy = deploy_fixture(&temp_dir("live-ok"));
    let (output, stdout) = check_live(&serve(deploy.clone(), 0), &deploy);
    assert!(output.status.success(), "{stdout}");
    assert!(!stdout.contains("待"), "待たなくてよいのに待った: {stdout}");
}

#[test]
fn a_page_whose_title_differs_from_the_deploy_dir_is_named() {
    let root = temp_dir("live-title");
    let served = deploy_fixture(&root.join("served"));
    let deploy = deploy_fixture(&root.join("expected"));
    write(
        deploy.join("data/concrete/index.html"),
        "<html><head><title>鉄筋 · site</title></head></html>",
    );
    let (output, stdout) = check_live(&serve(served, 0), &deploy);
    assert!(!output.status.success(), "{stdout}");
    assert!(
        stdout.contains("NG: /data/concrete/ の <title> が配信用ディレクトリと違う"),
        "{stdout}"
    );
    assert_eq!(stdout.matches("NG:").count(), 1, "{stdout}");
}

#[test]
fn a_not_found_whose_title_differs_from_the_deploy_dirs_404_html_is_named() {
    let root = temp_dir("live-404");
    let served = deploy_fixture(&root.join("served"));
    let deploy = deploy_fixture(&root.join("expected"));
    write(
        deploy.join("404.html"),
        "<html><head><title>Gone · site</title></head></html>",
    );
    let (output, stdout) = check_live(&serve(served, 0), &deploy);
    assert!(!output.status.success(), "{stdout}");
    assert!(
        stdout.contains("NG: 未知の住所に not-found の本文が無い"),
        "{stdout}"
    );
}

#[test]
fn files_that_404_just_after_a_deploy_are_fetched_again() {
    let deploy = deploy_fixture(&temp_dir("live-once"));
    let (output, stdout) = check_live(&serve(deploy.clone(), 1), &deploy);
    assert!(output.status.success(), "{stdout}");
    // / は最初の接続の確かめで 1 回目を使うので、待つのはそれ以外
    for waited in [
        "/assets/tailwind-dxh1.css は 1 秒待った",
        "/assets/rustruct-web-dxh1.js は 1 秒待った",
        "/assets/rustruct-web_bg-dxh1.wasm は 1 秒待った",
        "/data/ は 1 秒待った",
        "/data/concrete/ は 1 秒待った",
    ] {
        assert!(stdout.contains(waited), "「{waited}」が無い: {stdout}");
    }
}

#[test]
fn files_that_keep_404ing_fail_once_the_retries_run_out() {
    let deploy = deploy_fixture(&temp_dir("live-always"));
    let (output, stdout) = check_live(&serve(deploy.clone(), usize::MAX), &deploy);
    assert!(!output.status.success(), "{stdout}");
    // ファイルは名前順に見る。最初の 1 つで持ち時間を使い切ったら、残りは待たずに判定する
    assert!(
        stdout.contains("/assets/rustruct-web-dxh1.js は 12 秒待った"),
        "{stdout}"
    );
    assert!(
        stdout.contains("NG: /assets/rustruct-web-dxh1.js が 200 でない: 404"),
        "{stdout}"
    );
    assert!(stdout.contains("NG: /data/ が 200 でない: 404"), "{stdout}");
    assert!(
        !stdout.contains("/assets/rustruct-web_bg-dxh1.wasm は"),
        "持ち時間を使い切ったのにまた待った: {stdout}"
    );
}

#[test]
fn a_missing_deploy_dir_says_to_stage_first() {
    let deploy = temp_dir("live-none").join("deploy");
    let (output, stdout) = check_live("http://127.0.0.1:9", &deploy);
    assert!(!output.status.success(), "{stdout}");
    assert!(stdout.contains("stage-deploy.sh"), "{stdout}");
}
