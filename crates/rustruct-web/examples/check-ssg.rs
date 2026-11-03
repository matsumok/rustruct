//! SSG の出力のスモークテスト（Issue #12・#94）。`scripts/check.sh smoke` が `scripts/check-ssg.sh` 越しに呼ぶ。
//! 何を見るかは `rustruct_web::check_ssg_output`（src/ssg.rs）。
//!   cargo run -q -p rustruct-web --example check-ssg --locked [-- 出力ディレクトリ]
//!   既定: target/dx/rustruct-web/release/web/public

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/dx/rustruct-web/release/web/public")
        });
    let problems = rustruct_web::check_ssg_output(&out);
    for problem in &problems {
        println!("NG: {problem}");
    }
    if problems.is_empty() {
        println!(
            "OK: SSG の出力 {} は住所ごとの HTML と CSS・WASM を揃えている",
            out.display()
        );
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
