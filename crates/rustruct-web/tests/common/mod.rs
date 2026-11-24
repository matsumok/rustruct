//! テストで共有する道具。文字の順序を確かめ、一時ディレクトリに見本のファイルを置く。
//! HTML を読む道具（本文、要素の範囲、開きタグと属性）は `rustruct_web::markup` のものを渡す。

// common はテストのクレートごとに入り、どのクレートも一部しか使わないので、使わない道具と渡す名前で警告が出る
#![allow(dead_code, unused_imports)]

pub use rustruct_web::markup;
pub use rustruct_web::markup::{element, text_of};

use std::fs;
use std::path::{Path, PathBuf};

/// 読めない欄と理由を、理由の文字で比べられる形に
pub fn reasons<F: Copy>(errors: &[(F, String)]) -> Vec<(F, &str)> {
    errors
        .iter()
        .map(|(field, reason)| (*field, reason.as_str()))
        .collect()
}

/// `needles` が `haystack` にこの順で現れることを確かめる。無ければどれが無いかを言う。
pub fn assert_in_order(haystack: &str, needles: &[&str]) {
    let mut from = 0;
    for n in needles {
        let at = haystack[from..]
            .find(n)
            .unwrap_or_else(|| panic!("「{n}」が無い（またはこの順でない）: {haystack}"));
        from += at + n.len();
    }
}

/// リポジトリの直下（`scripts/` を呼ぶのに使う）。
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// 空の一時ディレクトリ。テストごとに別の名前にして、並列に走っても混ざらない。
/// 置き場は cargo が target ごとに用意する CARGO_TARGET_TMPDIR。作業用のフォルダ同士で混ざらず、
/// 共有の $TMPDIR（RAM の上で小さい）も使わない。プロセス番号はサンドボックスの中では毎回同じなので名前に使わない
pub fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("rustruct-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `path` に `content` を書く。途中のディレクトリも作る。
pub fn write(path: PathBuf, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}
