//! HTML の開きタグを、要素の名前と属性の並びに分ける。属性の順と、値の引用符の有無（Dioxus が
//! 値の型で変える。`aria-expanded=false` と `aria-pressed="true"`）に頼らずに要素を数える。
//! 結合テストは `common` から、`src` の単体テストは `lib.rs` の `#[path]` で同じものを使う。

// どのテストも一部しか使わないので、使わない道具で警告が出る
#![allow(dead_code)]

/// 開きタグ 1 つ。属性の値は書かれたまま（文字参照は戻さない）、値の無い属性は空の文字
pub struct Tag<'a> {
    pub name: &'a str,
    pub attributes: Vec<(&'a str, &'a str)>,
}

impl Tag<'_> {
    /// 属性の値。無ければ `None`
    pub fn get(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
    }

    /// `class` を空白で分けた並び
    pub fn classes(&self) -> impl Iterator<Item = &str> {
        self.get("class").unwrap_or("").split_whitespace()
    }
}

/// `html` の開きタグを順に。閉じタグ、コメント、`<!doctype>` は除く
pub fn tags(html: &str) -> Vec<Tag<'_>> {
    let bytes = html.as_bytes();
    let mut tags = Vec::new();
    let mut i = 0;
    while let Some(at) = html[i..].find('<') {
        i += at + 1;
        if !bytes.get(i).is_some_and(u8::is_ascii_alphabetic) {
            continue;
        }
        let name_end = run_until(html, i, |b| {
            b.is_ascii_whitespace() || b == b'>' || b == b'/'
        });
        let name = &html[i..name_end];
        i = name_end;
        let mut attributes = Vec::new();
        loop {
            i = run_until(html, i, |b| !b.is_ascii_whitespace() && b != b'/');
            if i >= bytes.len() || bytes[i] == b'>' {
                break;
            }
            let attr_end = run_until(html, i, |b| {
                b.is_ascii_whitespace() || b == b'=' || b == b'>' || b == b'/'
            });
            let attr = &html[i..attr_end];
            i = attr_end;
            let mut value = "";
            if bytes.get(i) == Some(&b'=') {
                i += 1;
                match bytes.get(i) {
                    Some(&quote @ (b'"' | b'\'')) => {
                        let end = run_until(html, i + 1, |b| b == quote);
                        value = &html[i + 1..end];
                        i = (end + 1).min(bytes.len());
                    }
                    _ => {
                        let end = run_until(html, i, |b| b.is_ascii_whitespace() || b == b'>');
                        value = &html[i..end];
                        i = end;
                    }
                }
            }
            attributes.push((attr, value));
        }
        tags.push(Tag { name, attributes });
    }
    tags
}

/// `name` の要素のうち、`attributes` の属性をすべてその値で持つものの数
pub fn count_tags(html: &str, name: &str, attributes: &[(&str, &str)]) -> usize {
    tags(html)
        .iter()
        .filter(|t| t.name == name && attributes.iter().all(|(n, v)| t.get(n) == Some(*v)))
        .count()
}

/// `from` から先で、`stop` に当たる最初のバイトの位置（無ければ末尾）。区切りはどれも ASCII なので、
/// 返す位置は文字の境目になる
fn run_until(html: &str, from: usize, stop: impl Fn(u8) -> bool) -> usize {
    html.as_bytes()[from.min(html.len())..]
        .iter()
        .position(|&b| stop(b))
        .map_or(html.len(), |at| from + at)
}
