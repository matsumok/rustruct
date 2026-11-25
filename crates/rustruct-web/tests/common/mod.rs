//! テストで共有する道具。文字の順序を確かめ、一時ディレクトリに見本のファイルを置く。
//! HTML を読む道具（本文、要素の範囲、開きタグと属性）は `rustruct_web::markup` のものを渡す。

// common はテストのクレートごとに入り、どのクレートも一部しか使わないので、使わない道具と渡す名前で警告が出る
#![allow(dead_code, unused_imports)]

pub use rustruct_web::InputRecord;
pub use rustruct_web::markup;
pub use rustruct_web::markup::{element, text_of};

use std::fs;
use std::path::{Path, PathBuf};

/// 入力の組の記録を、文字で比べられる形にしたもの（[`said`]）
#[derive(Debug, PartialEq)]
pub struct Said<'a, F> {
    /// 欄ごとの理由。欄の並びの順
    pub reasons: Vec<(F, &'a str)>,
    /// 欄をまたぐ理由と関わる欄
    pub across: Vec<(Vec<F>, &'a str)>,
    /// 警告（関わる欄は見ない。どの組に出るかはページの HTML で見る）
    pub warnings: Vec<&'a str>,
}

impl<F> Default for Said<'_, F> {
    /// 何も言わない記録
    fn default() -> Self {
        Self {
            reasons: Vec::new(),
            across: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// 記録が言うこと。3 ページのテストがこれで記録を比べる
pub fn said<F: Copy + PartialEq>(record: &InputRecord<F>) -> Said<'_, F> {
    Said {
        reasons: record
            .reasons()
            .iter()
            .map(|(field, reason)| (*field, reason.as_str()))
            .collect(),
        across: record
            .across()
            .iter()
            .map(|(fields, reason)| (fields.clone(), reason.as_str()))
            .collect(),
        warnings: record
            .warnings()
            .iter()
            .map(|(_, warning)| warning.as_str())
            .collect(),
    }
}

/// ページの HTML を入力の組ごとに。組の Sheet（欄を持つ `table`）から次の組の Sheet の手前まで（最後の組は
/// 終わりまで）で、組の注記・理由・警告と、組の下の結果の表を含む
pub fn input_groups(html: &str) -> Vec<&str> {
    let starts: Vec<usize> = markup::elements(html, "table")
        .into_iter()
        .filter(|table| table.contains("aria-label=\""))
        .map(|table| table.as_ptr() as usize - html.as_ptr() as usize)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| &html[start..starts.get(i + 1).copied().unwrap_or(html.len())])
        .collect()
}

/// 組の中の段落（`p`）の文字。理由（「欄の名前: 理由」と欄をまたぐ理由）、警告（「! …」）と、組の下の
/// 結果の段落（分布図の「—」など）
pub fn paragraphs(group: &str) -> Vec<String> {
    markup::elements(group, "p")
        .into_iter()
        .map(text_of)
        .collect()
}

/// 組の中の、`ng` の見出し（`th`）の数
pub fn flagged_heads(group: &str) -> usize {
    markup::count_tags(group, "th", &[("data-state", "error")])
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
