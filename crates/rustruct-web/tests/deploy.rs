//! 配信用ディレクトリの組み立て（`scripts/stage-deploy.sh`、Issue #13）。
//!
//! SSG は not-found を `404/index.html` に書く（dioxus-server は住所の下に必ず `index.html` を置く）が、
//! Workers Static Assets が未知の住所に返すのは `404.html`（`not_found_handling = "404-page"`）。
//! このスクリプトがその置き換えを行う。外から見えるのは出来上がったディレクトリだけなので、それを見る。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// 空の一時ディレクトリ。テストごとに別の名前にして、並列に走っても混ざらない。
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustruct-deploy-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: PathBuf, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// SSG 出力の形をした小さな見本。
fn ssg_fixture(root: &Path) -> PathBuf {
    let out = root.join("public");
    write(out.join("index.html"), "<title>Home</title>");
    write(out.join("tools/index.html"), "<title>Tools</title>");
    write(out.join("404/index.html"), "<p>この住所には何もない。</p>");
    write(out.join("assets/app.wasm"), "wasm bytes");
    out
}

fn stage(ssg_out: &Path, deploy: &Path) -> std::process::Output {
    Command::new("sh")
        .arg(workspace_root().join("scripts/stage-deploy.sh"))
        .arg(ssg_out)
        .arg(deploy)
        .output()
        .expect("sh が起動できない")
}

#[test]
fn not_found_moves_to_root_404_html_and_the_rest_is_kept() {
    let root = temp_dir("stage");
    let ssg_out = ssg_fixture(&root);
    let deploy = root.join("deploy");

    let output = stage(&ssg_out, &deploy);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fs::read_to_string(deploy.join("404.html")).unwrap(),
        "<p>この住所には何もない。</p>",
        "not-found はルートの 404.html に、中身そのままで置かれる"
    );
    assert!(
        !deploy.join("404").exists(),
        "404/ は残さない（同じ中身の住所を 2 つ作らない）"
    );
    assert_eq!(
        fs::read_to_string(deploy.join("index.html")).unwrap(),
        "<title>Home</title>"
    );
    assert_eq!(
        fs::read_to_string(deploy.join("tools/index.html")).unwrap(),
        "<title>Tools</title>"
    );
    assert_eq!(
        fs::read_to_string(deploy.join("assets/app.wasm")).unwrap(),
        "wasm bytes"
    );
    assert!(
        ssg_out.join("404/index.html").exists(),
        "SSG の出力そのものは変えない"
    );
}

#[test]
fn staging_twice_replaces_the_previous_deploy_dir() {
    let root = temp_dir("twice");
    let ssg_out = ssg_fixture(&root);
    let deploy = root.join("deploy");
    write(deploy.join("stale/index.html"), "前回の残り");

    let output = stage(&ssg_out, &deploy);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!deploy.join("stale").exists(), "前回の配信物は残さない");
    assert!(deploy.join("404.html").exists());
}

#[test]
fn fails_when_the_ssg_output_has_no_not_found_page() {
    let root = temp_dir("missing");
    let ssg_out = root.join("public");
    write(ssg_out.join("index.html"), "<title>Home</title>");
    let deploy = root.join("deploy");

    let output = stage(&ssg_out, &deploy);
    assert!(
        !output.status.success(),
        "not-found が無い出力は配信物にしない"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("404/index.html"),
        "何が無いかを言う: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
