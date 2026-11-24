//! 描いた HTML を読む道具。開きタグを要素の名前と属性の並びに分け、要素の範囲を名前の一致で切り出し、
//! タグを外して文字参照を戻した本文と `<title>` の中身を取る。属性の順と、値の引用符の有無（Dioxus は
//! 値の型で変える。`aria-expanded=false` と `aria-pressed="true"`。dx の出力にも `defer=true` がある）に頼らない。
//! SSG の出力の検査（`ssg.rs` の `output`）とテストが使う。

/// 開きタグ 1 つ。属性の値は書かれたまま（文字参照は戻さない）、値の無い属性は空の文字
pub struct Tag<'a> {
    pub name: &'a str,
    pub attributes: Vec<(&'a str, &'a str)>,
}

impl<'a> Tag<'a> {
    /// 属性の値。無ければ `None`
    pub fn get(&self, name: &str) -> Option<&'a str> {
        self.attributes
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
    }

    /// `class` を空白で分けた並び
    pub fn classes(&self) -> impl Iterator<Item = &'a str> {
        self.get("class").unwrap_or("").split_whitespace()
    }
}

/// `html` の開きタグを順に。閉じタグ、コメント、`<!doctype>`、`<script>` と `<style>` の中身は除く
pub fn tags(html: &str) -> Vec<Tag<'_>> {
    tokens(html)
        .into_iter()
        .filter_map(|token| match token {
            Token::Open { tag, .. } => Some(tag),
            Token::Close { .. } => None,
        })
        .collect()
}

/// `name` の要素のうち、`attributes` の属性をすべてその値で持つものの数
pub fn count_tags(html: &str, name: &str, attributes: &[(&str, &str)]) -> usize {
    tags(html)
        .iter()
        .filter(|t| t.name == name && attributes.iter().all(|(n, v)| t.get(n) == Some(*v)))
        .count()
}

/// 最初の `name` の要素（開きタグから閉じタグまで）。名前は全体で比べる（`head` は `<header>` に当たらない）。
/// 中に同じ名前の要素があれば、それを含めて外側の閉じタグまで。`/>` と空要素（`input` など）は開きタグだけ
///
/// # Panics
///
/// `name` の要素が無いか、閉じていないとき（テストで見たい要素が無いことを、そのまま失敗にする）
pub fn element<'a>(html: &'a str, name: &str) -> &'a str {
    let found = ranges(html, name)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("<{name}> が無いか閉じていない: {html}"));
    &html[found.outer]
}

/// `name` の要素を順に（入れ子の内側は外側に含め、別に数えない）
pub fn elements<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    ranges(html, name)
        .into_iter()
        .map(|found| &html[found.outer])
        .collect()
}

/// タグを外し、文字参照（`<` の `&#60;` など）を戻した本文
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

/// 最初の `<title>` の中身（書かれたまま）。無ければ `None`
pub fn title(html: &str) -> Option<&str> {
    ranges(html, "title")
        .into_iter()
        .next()
        .map(|found| &html[found.inner])
}

/// 中身を持たない要素。閉じタグを書かず、`/>` も要らない
const VOID: [&str; 13] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// 中身をタグとして読まない要素。閉じタグまで飛ばす
const RAW_TEXT: [&str; 2] = ["script", "style"];

/// タグ 1 つと、`html` の中の位置
enum Token<'a> {
    /// 開きタグ。`start..end` が `<` から `>` まで。`closed` は `/>` か空要素で、閉じタグを持たない
    Open {
        tag: Tag<'a>,
        start: usize,
        end: usize,
        closed: bool,
    },
    /// 閉じタグ。`start..end` が `</` から `>` まで
    Close {
        name: &'a str,
        start: usize,
        end: usize,
    },
}

/// 要素 1 つの範囲。`outer` は開きタグから閉じタグまで、`inner` はその間
struct Found {
    outer: std::ops::Range<usize>,
    inner: std::ops::Range<usize>,
}

/// `name` の要素の範囲を順に。閉じていないものは含めない
fn ranges(html: &str, name: &str) -> Vec<Found> {
    let mut found = Vec::new();
    // 外側の開きタグの (始まり, 終わり) と、同じ名前の入れ子の深さ
    let mut open: Option<(usize, usize, usize)> = None;
    for token in tokens(html) {
        match (token, &mut open) {
            (
                Token::Open {
                    tag,
                    start,
                    end,
                    closed,
                },
                None,
            ) if tag.name == name => {
                if closed {
                    found.push(Found {
                        outer: start..end,
                        inner: end..end,
                    });
                } else {
                    open = Some((start, end, 1));
                }
            }
            (Token::Open { tag, closed, .. }, Some((_, _, depth)))
                if tag.name == name && !closed =>
            {
                *depth += 1;
            }
            (
                Token::Close {
                    name: n,
                    start,
                    end,
                },
                Some((outer_start, inner_start, depth)),
            ) if n == name => {
                *depth -= 1;
                if *depth == 0 {
                    found.push(Found {
                        outer: *outer_start..end,
                        inner: *inner_start..start,
                    });
                    open = None;
                }
            }
            _ => {}
        }
    }
    found
}

/// `html` のタグを順に。コメントと `<!doctype>` は除く
fn tokens(html: &str) -> Vec<Token<'_>> {
    let bytes = html.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while let Some(at) = html[i..].find('<') {
        let start = i + at;
        i = start + 1;
        // コメントの中身（Dioxus の `<!--/-->` など）をタグとして読まない
        if html[start..].starts_with("<!--") {
            i = html[start + 4..]
                .find("-->")
                .map_or(bytes.len(), |at| start + 4 + at + 3);
            continue;
        }
        if bytes.get(i) == Some(&b'/') && bytes.get(i + 1).is_some_and(u8::is_ascii_alphabetic) {
            let name_end = run_until(html, i + 1, |b| b.is_ascii_whitespace() || b == b'>');
            let end = (run_until(html, name_end, |b| b == b'>') + 1).min(bytes.len());
            tokens.push(Token::Close {
                name: &html[i + 1..name_end],
                start,
                end,
            });
            i = end;
            continue;
        }
        if !bytes.get(i).is_some_and(u8::is_ascii_alphabetic) {
            continue;
        }
        let name_end = run_until(html, i, |b| {
            b.is_ascii_whitespace() || b == b'>' || b == b'/'
        });
        let name = &html[i..name_end];
        i = name_end;
        let mut attributes = Vec::new();
        let mut slash;
        loop {
            let gap_end = run_until(html, i, |b| !b.is_ascii_whitespace() && b != b'/');
            slash = html[i..gap_end].contains('/');
            i = gap_end;
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
        let end = (i + 1).min(bytes.len());
        let closed = slash || VOID.contains(&name);
        tokens.push(Token::Open {
            tag: Tag { name, attributes },
            start,
            end,
            closed,
        });
        i = end;
        // script と style の中身（`a<b` など）をタグとして読まない
        if !closed && RAW_TEXT.contains(&name) {
            let close = format!("</{name}");
            i = html[i..].find(&close).map_or(bytes.len(), |at| i + at);
        }
    }
    tokens
}

/// `from` から先で、`stop` に当たる最初のバイトの位置（無ければ末尾）。区切りはどれも ASCII なので、
/// 返す位置は文字の境目になる
fn run_until(html: &str, from: usize, stop: impl Fn(u8) -> bool) -> usize {
    html.as_bytes()[from.min(html.len())..]
        .iter()
        .position(|&b| stop(b))
        .map_or(html.len(), |at| from + at)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// dx の出力の形を縮めた例。引用符の無い値、値の無い属性、`/>`、文字参照、`<head>` と `<header>`
    const PAGE: &str = concat!(
        "<!DOCTYPE html><html><head><title>Data · rustruct.matsumok.com</title>",
        r#"<meta name="description" content="材料と規格の値。"/>"#,
        r#"<script src="/assets/focus-trap.js" defer=true></script></head>"#,
        r#"<body><header><a href="/">rustruct</a></header>"#,
        r#"<main><input value=24 aria-label="Fc" disabled/><p>a &#60; b &amp;&gt; c</p></main></body></html>"#,
    );

    #[test]
    fn an_unquoted_value_reads_the_same_as_a_quoted_one() {
        let script = tags(PAGE).into_iter().find(|t| t.name == "script").unwrap();
        assert_eq!(script.get("defer"), Some("true"));
        assert_eq!(script.get("src"), Some("/assets/focus-trap.js"));
        assert_eq!(count_tags(PAGE, "input", &[("value", "24")]), 1);
    }

    #[test]
    fn an_attribute_without_a_value_is_present_with_an_empty_value() {
        let input = tags(PAGE).into_iter().find(|t| t.name == "input").unwrap();
        assert_eq!(input.get("disabled"), Some(""));
        assert_eq!(input.get("aria-label"), Some("Fc"));
        assert_eq!(input.get("checked"), None);
    }

    #[test]
    fn attributes_are_found_whatever_their_order() {
        for html in [
            r#"<a href="/data" class="text-ink">Data</a>"#,
            r#"<a class="text-ink" href=/data>Data</a>"#,
        ] {
            assert_eq!(
                count_tags(html, "a", &[("href", "/data"), ("class", "text-ink")]),
                1,
                "{html}"
            );
        }
    }

    #[test]
    fn an_element_is_found_by_its_whole_name_not_a_prefix() {
        assert!(
            element(PAGE, "head").starts_with("<head><title>"),
            "{}",
            element(PAGE, "head")
        );
        assert!(element(PAGE, "head").ends_with("</script></head>"));
        assert_eq!(
            element(PAGE, "header"),
            r#"<header><a href="/">rustruct</a></header>"#
        );
        assert_eq!(count_tags(PAGE, "head", &[]), 1);
        assert_eq!(count_tags(PAGE, "a", &[]), 1);
    }

    #[test]
    fn an_element_takes_in_the_same_name_nested_inside_it() {
        let html = "<div><div>a</div>b</div><div>c</div>";
        assert_eq!(element(html, "div"), "<div><div>a</div>b</div>");
        assert_eq!(
            elements(html, "div"),
            ["<div><div>a</div>b</div>", "<div>c</div>"]
        );
    }

    #[test]
    fn a_self_closing_element_is_only_its_tag() {
        assert_eq!(
            element(PAGE, "input"),
            r#"<input value=24 aria-label="Fc" disabled/>"#
        );
    }

    #[test]
    fn text_drops_the_tags_and_restores_character_references() {
        assert_eq!(text_of(element(PAGE, "main")), "a < b &> c");
    }

    #[test]
    fn the_title_is_what_the_title_element_holds() {
        assert_eq!(title(PAGE), Some("Data · rustruct.matsumok.com"));
        assert_eq!(title("<head></head>"), None);
    }

    #[test]
    fn a_comment_is_not_read_as_tags() {
        let html = "<p>a<!--<a href=/x>--></p><!--/--><a href=/y>b</a>";
        assert_eq!(count_tags(html, "a", &[]), 1);
        assert_eq!(element(html, "a"), "<a href=/y>b</a>");
    }
}
