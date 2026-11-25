//! 本文の Markdown を HTML にする（ADR site-0019）。説明書（`GLOSSARY.md`）の本文を、ビルドのときに `build.rs` が
//! `content/` のファイルごとに通す。ブラウザでは Markdown も KaTeX も動かさない。式は KaTeX の MathML だけで組み、
//! CSS もフォントも足さない。
//!
//! 扱う書き方は、見出し（`##`・`###`）、段落、箇条書き、番号つきの箇条、表、式（文中の `$…$` と別行の `$$…$$`）、
//! 文中のコード。ほかの書き方（画像、リンク、強調、`#` や `####` の見出し、HTML など）は `Err` で止め、黙って捨てない。
//!
//! 組み方は `docs/design/README.md` の「説明書の本文」。要素の間の空きは親の `gap` で持つので、見出しで節に分ける:
//! `##` の節は見出しと中身、`###` の節も同じで、節の中の段落・箇条・表・式は `space-3` で並べ、節と節は `space-7`。
//! 節の外の段落の並びも 1 つの組にする。並べる外側（本文の `article`）は `prose.rs` が持つ。

use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag as MdTag, TagEnd};

/// 書き出す要素の名前とクラス。クラスは `docs/design/README.md` の役割名だけ
struct Tag {
    name: &'static str,
    class: &'static str,
}

/// 見出しのある節と、節の外の段落の並び。中は見出しと中身で、セクション見出しと中身の間の `space-3`
const SECTION: Tag = Tag {
    name: "section",
    class: "flex flex-col gap-3",
};
/// 段落・箇条・表・式の並び。`space-3`
const RUN: Tag = Tag {
    name: "div",
    class: "flex flex-col gap-3",
};
/// `###` の節を含む `##` の節の中身。節と節の間は `space-7`
const SUBSECTIONS: Tag = Tag {
    name: "div",
    class: "flex flex-col gap-7",
};
/// `##` の見出し。セクションの見出しなので上に `ink` の線（`/tools` の分野の見出しと同じ）
const H2: Tag = Tag {
    name: "h2",
    class: "border-t border-ink pt-2 text-heading",
};
/// `###` の見出し。表や式の名前と同じ `heading`
const H3: Tag = Tag {
    name: "h3",
    class: "text-heading",
};
const P: Tag = Tag {
    name: "p",
    class: "",
};
const UL: Tag = Tag {
    name: "ul",
    class: "list-disc pl-5",
};
const OL: Tag = Tag {
    name: "ol",
    class: "list-decimal pl-5",
};
const LI: Tag = Tag {
    name: "li",
    class: "",
};
const CODE: Tag = Tag {
    name: "code",
    class: "font-mono",
};
/// 別行の式。はみ出すときは式だけが横に動く。括弧や分数の字は式の箱の上下に少しはみ出し、横に動く箱は縦にも
/// 動くようになるので、上下に `space-2`（行の上下）を取って字を箱の中に収める
const DISPLAY: Tag = Tag {
    name: "div",
    class: "overflow-x-auto py-2",
};
/// 表は `docs/design/README.md` の「表」の形（`table.rs` の `Table` と同じ）。1単位に入りきらなければ表だけが横に動く
const TABLE_SCROLL: Tag = Tag {
    name: "div",
    class: "overflow-x-auto",
};
const TABLE: Tag = Tag {
    name: "table",
    class: "w-full",
};
const THEAD: Tag = Tag {
    name: "thead",
    class: "",
};
const TBODY: Tag = Tag {
    name: "tbody",
    class: "",
};
const TR: Tag = Tag {
    name: "tr",
    class: "border-b border-line",
};
const TH: Tag = Tag {
    name: "th",
    class: "py-2 pl-3 first:pl-0 first:pr-3 text-left text-label text-muted font-normal whitespace-nowrap",
};
const TH_RIGHT: Tag = Tag {
    name: "th",
    class: "py-2 pl-3 first:pl-0 first:pr-3 text-right text-label text-muted font-normal whitespace-nowrap",
};
const TD: Tag = Tag {
    name: "td",
    class: "py-2 pl-3 first:pl-0 first:pr-3 text-body",
};
const TD_RIGHT: Tag = Tag {
    name: "td",
    class: "py-2 pl-3 first:pl-0 first:pr-3 text-right text-body",
};

impl Tag {
    fn open(&self, out: &mut String) {
        out.push('<');
        out.push_str(self.name);
        if !self.class.is_empty() {
            out.push_str(" class=\"");
            out.push_str(self.class);
            out.push('"');
        }
        out.push('>');
    }

    fn close(&self, out: &mut String) {
        out.push_str("</");
        out.push_str(self.name);
        out.push('>');
    }
}

/// `markdown` の本文の HTML。並べる外側の要素は含めない（節と段落の並びを順に）。
/// 扱わない書き方と、組めない式は `Err`（何が、どこで）
pub fn to_html(markdown: &str) -> Result<String, String> {
    let pieces = pieces(markdown)?;
    let items = nest(pieces)?;
    let mut out = String::new();
    write_items(&items, &mut out);
    Ok(out)
}

/// 見出しと、見出しの間の 1 つずつの塊（段落・箇条・表・式）
enum Piece {
    Heading(u8, String),
    Block(String),
}

/// 節に分けた本文。塊か、見出しと中身の節
enum Item {
    Block(String),
    Section {
        level: u8,
        heading: String,
        items: Vec<Item>,
    },
}

/// 文書を見出しと塊に分ける
fn pieces(markdown: &str) -> Result<Vec<Piece>, String> {
    let mut parser = Parser::new_ext(markdown, Options::ENABLE_TABLES | Options::ENABLE_MATH);
    let mut pieces = Vec::new();
    while let Some(event) = parser.next() {
        let piece = match event {
            Event::Start(MdTag::Heading { level, .. }) => {
                let level = match level {
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    other => return Err(format!("見出しは ## と ### だけ: {other:?}")),
                };
                Piece::Heading(
                    level,
                    phrasing_until(&mut parser, TagEnd::Heading(heading_level(level)))?,
                )
            }
            Event::Start(MdTag::Paragraph) => Piece::Block(paragraph(&mut parser)?),
            Event::Start(MdTag::List(first)) => {
                let mut out = String::new();
                list(&mut parser, first, &mut out)?;
                Piece::Block(out)
            }
            Event::Start(MdTag::Table(alignments)) => {
                Piece::Block(table(&mut parser, &alignments)?)
            }
            other => return Err(unsupported(&other)),
        };
        pieces.push(piece);
    }
    Ok(pieces)
}

fn heading_level(level: u8) -> HeadingLevel {
    if level == 2 {
        HeadingLevel::H2
    } else {
        HeadingLevel::H3
    }
}

/// 段落。別行の式だけの段落は式の塊にし、`<p>` に入れない。別行の式を文と混ぜた段落は `Err`
fn paragraph(parser: &mut Parser) -> Result<String, String> {
    let mut events = Vec::new();
    for event in parser.by_ref() {
        if event == Event::End(TagEnd::Paragraph) {
            break;
        }
        events.push(event);
    }
    let display: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::DisplayMath(tex) => Some(tex.as_ref()),
            _ => None,
        })
        .collect();
    if display.is_empty() {
        let mut out = String::new();
        P.open(&mut out);
        for event in &events {
            phrasing(event, &mut out)?;
        }
        P.close(&mut out);
        return Ok(out);
    }
    let only_math = events.iter().all(|e| match e {
        Event::DisplayMath(_) | Event::SoftBreak => true,
        Event::Text(text) => text.trim().is_empty(),
        _ => false,
    });
    if !only_math {
        return Err(format!(
            "別行の式（$$…$$）は文と混ぜず、段落を分ける: {}",
            display.join(" / ")
        ));
    }
    let mut out = String::new();
    for tex in display {
        DISPLAY.open(&mut out);
        out.push_str(&math(tex, true)?);
        DISPLAY.close(&mut out);
    }
    Ok(out)
}

/// 箇条（番号つきなら `first` が最初の番号。1 から始める）
fn list(parser: &mut Parser, first: Option<u64>, out: &mut String) -> Result<(), String> {
    let tag = match first {
        None => &UL,
        Some(1) => &OL,
        Some(n) => return Err(format!("番号つきの箇条は 1 から始める: {n}")),
    };
    tag.open(out);
    while let Some(event) = parser.next() {
        match event {
            Event::Start(MdTag::Item) => {
                LI.open(out);
                item(parser, out)?;
                LI.close(out);
            }
            Event::End(TagEnd::List(_)) => break,
            other => return Err(unsupported(&other)),
        }
    }
    tag.close(out);
    Ok(())
}

/// 箇条の 1 項。文と、入れ子の箇条
fn item(parser: &mut Parser, out: &mut String) -> Result<(), String> {
    while let Some(event) = parser.next() {
        match event {
            Event::End(TagEnd::Item) => return Ok(()),
            Event::Start(MdTag::List(first)) => list(parser, first, out)?,
            Event::Start(MdTag::Paragraph) => out.push_str(&paragraph(parser)?),
            other => phrasing(&other, out)?,
        }
    }
    Ok(())
}

/// 表。揃えは左（既定）と右だけ
fn table(parser: &mut Parser, alignments: &[Alignment]) -> Result<String, String> {
    let right: Vec<bool> = alignments
        .iter()
        .map(|a| match a {
            Alignment::None | Alignment::Left => Ok(false),
            Alignment::Right => Ok(true),
            Alignment::Center => Err("表の列の揃えは左か右だけ（中央は使わない）".to_string()),
        })
        .collect::<Result<_, _>>()?;
    let mut out = String::new();
    TABLE_SCROLL.open(&mut out);
    TABLE.open(&mut out);
    let mut head = true;
    let mut column = 0;
    while let Some(event) = parser.next() {
        match event {
            Event::Start(MdTag::TableHead) => {
                THEAD.open(&mut out);
                TR.open(&mut out);
                column = 0;
            }
            Event::End(TagEnd::TableHead) => {
                TR.close(&mut out);
                THEAD.close(&mut out);
                TBODY.open(&mut out);
                head = false;
            }
            Event::Start(MdTag::TableRow) => {
                TR.open(&mut out);
                column = 0;
            }
            Event::End(TagEnd::TableRow) => TR.close(&mut out),
            Event::Start(MdTag::TableCell) => {
                let tag = match (head, right.get(column).copied().unwrap_or(false)) {
                    (true, false) => &TH,
                    (true, true) => &TH_RIGHT,
                    (false, false) => &TD,
                    (false, true) => &TD_RIGHT,
                };
                tag.open(&mut out);
                out.push_str(&phrasing_until(parser, TagEnd::TableCell)?);
                tag.close(&mut out);
                column += 1;
            }
            Event::End(TagEnd::Table) => break,
            other => return Err(unsupported(&other)),
        }
    }
    if !head {
        TBODY.close(&mut out);
    }
    TABLE.close(&mut out);
    TABLE_SCROLL.close(&mut out);
    Ok(out)
}

/// `end` までの文中の要素
fn phrasing_until(parser: &mut Parser, end: TagEnd) -> Result<String, String> {
    let mut out = String::new();
    for event in parser.by_ref() {
        if event == Event::End(end) {
            return Ok(out);
        }
        phrasing(&event, &mut out)?;
    }
    Ok(out)
}

/// 文中の要素 1 つ: 文字、文中のコード、文中の式、行の折り返し
fn phrasing(event: &Event, out: &mut String) -> Result<(), String> {
    match event {
        Event::Text(text) => out.push_str(&escape(text)),
        Event::Code(code) => {
            CODE.open(out);
            out.push_str(&escape(code));
            CODE.close(out);
        }
        Event::InlineMath(tex) => out.push_str(&math(tex, false)?),
        Event::SoftBreak => out.push('\n'),
        Event::DisplayMath(tex) => {
            return Err(format!("別行の式（$$…$$）は段落に 1 つで置く: {tex}"));
        }
        other => return Err(unsupported(other)),
    }
    Ok(())
}

/// KaTeX が組んだ式の MathML（`<math>` の要素だけ。KaTeX が外に付ける `span` は外す）
fn math(tex: &str, display: bool) -> Result<String, String> {
    let opts = katex::Opts::builder()
        .display_mode(display)
        .output_type(katex::OutputType::Mathml)
        .throw_on_error(true)
        .build()
        .map_err(|e| format!("KaTeX の設定: {e}"))?;
    let html =
        katex::render_with_opts(tex, &opts).map_err(|e| format!("式を組めない（{tex}）: {e}"))?;
    let start = html
        .find("<math")
        .ok_or_else(|| format!("KaTeX の出力に <math> が無い: {html}"))?;
    let end = html
        .rfind("</math>")
        .ok_or_else(|| format!("KaTeX の出力に </math> が無い: {html}"))?;
    Ok(html[start..end + "</math>".len()].to_string())
}

/// 見出しと塊を節に入れる。`###` は `##` の節の中にだけ置ける
fn nest(pieces: Vec<Piece>) -> Result<Vec<Item>, String> {
    let mut top: Vec<Item> = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Heading(2, heading) => top.push(Item::Section {
                level: 2,
                heading,
                items: Vec::new(),
            }),
            Piece::Heading(level, heading) => {
                let Some(Item::Section { items, .. }) = top.last_mut() else {
                    return Err(format!("### の見出しは ## の節の中に置く: {heading}"));
                };
                items.push(Item::Section {
                    level,
                    heading,
                    items: Vec::new(),
                });
            }
            Piece::Block(block) => {
                let innermost = match top.last_mut() {
                    Some(Item::Section { items, .. }) => match items.last_mut() {
                        Some(Item::Section { items, .. }) => items,
                        _ => items,
                    },
                    _ => &mut top,
                };
                innermost.push(Item::Block(block));
            }
        }
    }
    Ok(top)
}

/// 節と塊の並び。続く塊は 1 つの組（`RUN`）にまとめる
fn write_items(items: &[Item], out: &mut String) {
    let mut run = false;
    for item in items {
        match item {
            Item::Block(block) => {
                if !run {
                    RUN.open(out);
                    run = true;
                }
                out.push_str(block);
            }
            Item::Section {
                level,
                heading,
                items,
            } => {
                if run {
                    RUN.close(out);
                    run = false;
                }
                write_section(*level, heading, items, out);
            }
        }
    }
    if run {
        RUN.close(out);
    }
}

/// 見出しと中身の節。中身が塊だけなら見出しの下に並べ、`###` の節を含むなら節と組を `SUBSECTIONS` に並べる
fn write_section(level: u8, heading: &str, items: &[Item], out: &mut String) {
    SECTION.open(out);
    let tag = if level == 2 { &H2 } else { &H3 };
    tag.open(out);
    out.push_str(heading);
    tag.close(out);
    if items.iter().all(|i| matches!(i, Item::Block(_))) {
        for item in items {
            if let Item::Block(block) = item {
                out.push_str(block);
            }
        }
    } else {
        SUBSECTIONS.open(out);
        write_items(items, out);
        SUBSECTIONS.close(out);
    }
    SECTION.close(out);
}

/// 扱わない書き方の 1 行
fn unsupported(event: &Event) -> String {
    format!("扱わない書き方: {event:?}")
}

/// 文字のエスケープ
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::to_html;
    use crate::markup::{count_tags, element, elements, tags, text_of};

    fn html(markdown: &str) -> String {
        to_html(markdown).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn headings_open_sections_and_the_lower_heading_sits_inside_the_upper() {
        let out = html(
            "## 仮定と式\n\n前の段落\n\n### 仮定\n\n仮定の段落\n\n### 曲げ剛性\n\n剛性の段落\n",
        );
        let outer = element(&out, "section");
        assert_eq!(text_of(element(outer, "h2")), "仮定と式");
        let inner = elements(outer.strip_prefix("<section").unwrap(), "section");
        let names: Vec<String> = inner.iter().map(|s| text_of(element(s, "h3"))).collect();
        assert_eq!(names, ["仮定", "曲げ剛性"]);
        assert_eq!(text_of(element(inner[0], "p")), "仮定の段落");
        assert_eq!(text_of(element(inner[1], "p")), "剛性の段落");
        // ### の前の段落は ## の節の中で、### の節の外
        assert!(outer.find("前の段落").unwrap() < outer.find("<h3").unwrap());
    }

    #[test]
    fn a_table_has_its_head_cells_and_body_cells_in_rows() {
        let out = html("| 記号 | 量 |\n|---|---:|\n| $w$ | たわみ |\n| `Mx` | 曲げモーメント |\n");
        let table = element(&out, "table");
        let heads: Vec<String> = elements(element(table, "thead"), "th")
            .into_iter()
            .map(text_of)
            .collect();
        assert_eq!(heads, ["記号", "量"]);
        let rows: Vec<Vec<String>> = elements(element(table, "tbody"), "tr")
            .into_iter()
            .map(|tr| elements(tr, "td").into_iter().map(text_of).collect())
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][1], "たわみ");
        assert_eq!(rows[1], ["Mx", "曲げモーメント"]);
        // 式のセルは MathML で、コードのセルは code
        assert_eq!(count_tags(element(table, "tbody"), "math", &[]), 1);
        assert_eq!(text_of(element(table, "code")), "Mx");
    }

    #[test]
    fn inline_math_stays_in_the_sentence_and_display_math_stands_alone() {
        let out = html("曲げ剛性 $D$ を使う。\n\n$$D \\nabla^4 w = p$$\n");
        let p = element(&out, "p");
        assert!(text_of(p).starts_with("曲げ剛性"), "{p}");
        let in_text = tags(p)
            .into_iter()
            .find(|t| t.name == "math")
            .expect("文中の式が無い");
        assert_eq!(in_text.get("display"), None, "{p}");
        // 別行の式は段落の外で、display="block"。式は組んであり、TeX の文字は注釈にだけ残る
        assert_eq!(count_tags(&out, "p", &[]), 1, "{out}");
        assert_eq!(
            count_tags(&out, "math", &[("display", "block")]),
            1,
            "{out}"
        );
        assert!(!p.contains("\\nabla"), "{p}");
        assert!(count_tags(&out, "mo", &[]) > 0, "式が組まれていない: {out}");
    }

    #[test]
    fn lists_keep_their_kind_and_items() {
        let out = html("- 一つ目\n- 二つ目\n\n1. 最初\n2. 次\n");
        let items: Vec<String> = elements(element(&out, "ul"), "li")
            .into_iter()
            .map(text_of)
            .collect();
        assert_eq!(items, ["一つ目", "二つ目"]);
        let items: Vec<String> = elements(element(&out, "ol"), "li")
            .into_iter()
            .map(text_of)
            .collect();
        assert_eq!(items, ["最初", "次"]);
    }

    #[test]
    fn text_is_escaped() {
        assert!(html("a < b & c\n").contains("a &lt; b &amp; c"));
    }

    #[test]
    fn what_is_not_handled_stops_with_a_reason() {
        for markdown in [
            "# 見出し\n",
            "#### 見出し\n",
            "### 上の節が無い\n",
            "![図](a.png)\n",
            "[リンク](/tools)\n",
            "*強調*\n",
            "文と $$x$$ を混ぜる\n",
            "| a |\n|:-:|\n| b |\n",
            "2. 二から\n",
            "$\\frac{1}$\n",
        ] {
            assert!(to_html(markdown).is_err(), "通ってしまった: {markdown}");
        }
    }
}
