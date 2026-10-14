//! SSG 出力のスモークテスト（`scripts/check-ssg.sh`、Issue #12）そのものを確かめる。
//! 見本は `static_paths` の住所ごとに `render_path` の HTML を置き、dx が足す読込スクリプトと CSS・WASM を加えたもの。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use rustruct_web::{render_path, static_paths};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// 空の一時ディレクトリ。テストごとに別の名前にして、並列に走っても混ざらない。
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustruct-ssg-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: PathBuf, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// SSG 出力の形をした見本。CSS・読込スクリプト・WASM が 1 つずつで、参照が揃っている。
fn ssg_fixture(root: &Path) -> PathBuf {
    let out = root.join("public");
    let head = concat!(
        r#"<link rel="stylesheet" href="/assets/tailwind-dxh1.css">"#,
        r#"<script type="module" async src="/./assets/rustruct-web-dxh1.js"></script></head>"#,
    );
    for path in static_paths() {
        let html = render_path(&path).replacen("</head>", head, 1);
        write(
            out.join(path.trim_start_matches('/')).join("index.html"),
            &html,
        );
    }
    write(out.join("assets/tailwind-dxh1.css"), "");
    write(
        out.join("assets/rustruct-web-dxh1.js"),
        r#"fetch("/./assets/rustruct-web_bg-dxh1.wasm")"#,
    );
    write(out.join("assets/rustruct-web_bg-dxh1.wasm"), "");
    out
}

fn check(out: &Path) -> (Output, String) {
    let output = Command::new("sh")
        .arg(workspace_root().join("scripts/check-ssg.sh"))
        .arg(out)
        .output()
        .expect("sh が起動できない");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output, stdout)
}

#[test]
fn a_complete_ssg_output_passes() {
    let out = ssg_fixture(&temp_dir("ok"));
    let (output, stdout) = check(&out);
    assert!(output.status.success(), "{stdout}");
}

#[test]
fn a_stale_wasm_left_by_an_earlier_build_is_named_as_such() {
    // 手元で dx build を重ねると、前のビルドのハッシュ付きアセットが public/assets に残る（2026-10-14）
    let out = ssg_fixture(&temp_dir("stale"));
    write(out.join("assets/rustruct-web_bg-dxh0.wasm"), "");
    let (output, stdout) = check(&out);
    assert!(!output.status.success(), "{stdout}");
    assert!(stdout.contains("NG: WASM が 1 つでない"), "{stdout}");
    assert!(
        !stdout.contains("を参照していない"),
        "WASM が 2 つのときに参照の NG まで出ている: {stdout}"
    );
}
