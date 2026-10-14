//! テストで共有する道具。HTML から本文を取り出し、文字の順序を確かめる。

/// タグを外し、文字参照（`<` の `&#60;` など）を戻した本文。
pub fn text_of(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&#60;", "<")
        .replace("&lt;", "<")
        .replace("&#62;", ">")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
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
