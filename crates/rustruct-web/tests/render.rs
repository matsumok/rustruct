//! 継ぎ目 A（Issue #8 / #11）: 「パス → HTML 文字列」の関数 `render_path` を外から見る。
//! クラス名の並びは見ない。本文の文字と順序、どこで要素が分かれているか、`<title>` だけを見る。

mod common;
use common::{assert_in_order, element, text_of};
use rustruct_web::{NOT_FOUND_PATH, render_path, static_paths};

#[test]
fn wordmark_is_split_so_the_dot_and_domain_can_be_muted() {
    let html = render_path("/");
    // .matsumok.com がドットごと別の要素に入っている（muted にするため）
    let domain = html
        .find(">.matsumok.com<")
        .expect(".matsumok.com が単独の要素でない");
    let rustruct = html.find("rustruct").expect("rustruct が無い");
    assert!(rustruct < domain, "順序が違う: {html}");
}

#[test]
fn home_lists_the_four_sections_in_order_with_name_blurb_and_path() {
    let html = render_path("/");
    let text = text_of(&html);
    // 四部がこの順。各項目はパス、名前、一行の順に並ぶ
    assert_in_order(
        &text,
        &[
            "/tools",
            "Tools",
            "構造計算の入力・計算・判定。",
            "/data",
            "Data",
            "構造設計で参照する材料や規格の値。",
            "/studies",
            "Studies",
            "数学やかたちを眺めて考えるための可視化。",
            "/notes",
            "Notes",
            "構造や数学についての書きもの。",
        ],
    );
}

#[test]
fn home_items_link_to_their_section() {
    let html = render_path("/");
    for path in ["/tools", "/data", "/studies", "/notes"] {
        assert!(
            html.contains(&format!("href=\"{path}\"")),
            "{path} へのリンクが無い: {html}"
        );
    }
}

#[test]
fn home_shows_the_site_address_in_display_above_the_list() {
    let html = render_path("/");
    let text = text_of(element(&html, "main"));
    assert!(
        text.starts_with("rustruct.matsumok.com"),
        "トップの一文が住所でない: {text}"
    );
    assert_in_order(&text, &["rustruct.matsumok.com", "/tools"]);
}

#[test]
fn section_page_has_path_title_and_the_empty_sentence_only() {
    for (path, title) in [
        ("/tools", "Tools"),
        ("/studies", "Studies"),
        ("/notes", "Notes"),
    ] {
        let html = render_path(path);
        let text = text_of(&html);
        // 枠の wordmark の後に、伸びた住所、見出し、一文の順
        assert_in_order(
            &text,
            &["rustruct.matsumok.com", path, title, "まだ何もない。"],
        );
        assert_eq!(h1_text(&html), title, "{path} の見出し: {html}");
        // 本文に一覧（他の部の名前）は出ない。ヘッダーの四部のボタンは別
        let body = text_of(element(&html, "main"));
        for other in ["Tools", "Data", "Studies", "Notes"] {
            if other != title {
                assert!(
                    !body.contains(other),
                    "{path} の本文に {other} が出ている: {body}"
                );
            }
        }
    }
}

/// ページの `<h1>` の文字。1ページに1つ。
fn h1_text(html: &str) -> String {
    let start = html.find("<h1").expect("h1 が無い");
    let end = html[start..].find("</h1>").expect("h1 が閉じていない") + start;
    assert!(!html[end..].contains("<h1"), "h1 が2つ以上ある: {html}");
    text_of(&html[start..end])
}

/// トップの一覧の各項目: (href, リンクの中の文字)。本文（`<main>`）の中のリンクだけ。
fn home_items(home: &str) -> Vec<(String, String)> {
    let mut items = Vec::new();
    let mut rest = element(home, "main");
    while let Some(at) = rest.find("<a ") {
        let a = &rest[at..];
        let close = a.find("</a>").expect("a が閉じていない");
        let href_at = a.find("href=\"").expect("href が無い") + "href=\"".len();
        let href = &a[href_at..href_at + a[href_at..].find('"').expect("href が閉じていない")];
        items.push((href.to_string(), text_of(&a[..close])));
        rest = &a[close..];
    }
    items
}

#[test]
fn every_item_on_home_leads_to_a_page_headed_with_the_items_name() {
    let items = home_items(&render_path("/"));
    assert_eq!(items.len(), 4, "一覧の項目数");
    for (href, item_text) in items {
        // 項目の文字は「パス 名前 一行」の順なので、パスを除いた先頭が名前
        let after_path = item_text
            .strip_prefix(&href)
            .unwrap_or_else(|| panic!("項目がパス {href} で始まらない: {item_text}"));
        let heading = h1_text(&render_path(&href));
        assert!(
            after_path.starts_with(&heading),
            "{href} の見出し「{heading}」が一覧の名前と違う: {item_text}"
        );
    }
}

#[test]
fn root_is_an_empty_page_reserved_for_root() {
    let html = render_path("/root");
    let text = text_of(&html);
    assert_in_order(
        &text,
        &[
            "rustruct.matsumok.com",
            "/root",
            "root",
            "root のための場所。",
        ],
    );
    assert_eq!(h1_text(&html), "root");
    let body = text_of(element(&html, "main"));
    assert!(!body.contains("Tools"), "/root に一覧が出ている: {body}");
}

#[test]
fn unknown_path_is_not_found_inside_the_same_frame() {
    let html = render_path("/no/such/page");
    let text = text_of(&html);
    // wordmark（枠）があり、見出しと、「ない」ことの一文がある
    assert_in_order(
        &text,
        &[
            "rustruct.matsumok.com",
            "Not found",
            "この住所には何もない。",
        ],
    );
    assert_eq!(h1_text(&html), "Not found");
    assert!(
        html.contains("href=\"/\""),
        "wordmark からトップへ戻れない: {html}"
    );
    assert!(
        !text.contains("まだ何もない"),
        "not-found が空ページの文言になっている: {text}"
    );
}

/// not-found は静的な 1 ファイル（`/404`）として書き出され、どの住所で開かれても同じ中身が届く。
/// 求められた住所はブラウザでしか決まらないので、初回描画には入れない（ADR-0001 の規則）。
#[test]
fn not_found_html_is_the_same_whatever_path_was_asked_for() {
    let at_404 = render_path(NOT_FOUND_PATH);
    for path in ["/no/such/page", "/tools/nothing"] {
        assert_eq!(
            render_path(path),
            at_404,
            "{path} の not-found が /404 と違う"
        );
    }
    assert!(
        !text_of(&at_404).contains("404"),
        "not-found の本文に置き場の住所 /404 が出ている: {at_404}"
    );
}

#[test]
fn ssg_paths_are_the_static_routes_and_the_not_found_page() {
    assert_eq!(
        static_paths(),
        [
            "/",
            "/tools",
            "/data",
            "/data/concrete",
            "/data/rebar",
            "/studies",
            "/notes",
            "/root",
            "/404"
        ]
    );
}

#[test]
fn root_is_not_listed_on_home() {
    let text = text_of(&render_path("/"));
    assert!(!text.contains("/root"), "トップに /root が出ている: {text}");
}

/// `<head>` の中身。
fn head_of(html: &str) -> &str {
    let start = html.find("<head>").expect("head が無い");
    let end = html.find("</head>").expect("head が閉じていない");
    &html[start..end]
}

#[test]
fn title_is_page_name_then_site_name() {
    for (path, title) in [
        ("/", "Home · rustruct.matsumok.com"),
        ("/tools", "Tools · rustruct.matsumok.com"),
        ("/data", "Data · rustruct.matsumok.com"),
        ("/data/concrete", "コンクリート · rustruct.matsumok.com"),
        ("/data/rebar", "鉄筋 · rustruct.matsumok.com"),
        ("/studies", "Studies · rustruct.matsumok.com"),
        ("/notes", "Notes · rustruct.matsumok.com"),
        ("/root", "root · rustruct.matsumok.com"),
        ("/no/such/page", "Not found · rustruct.matsumok.com"),
    ] {
        let html = render_path(path);
        assert!(
            head_of(&html).contains(&format!("<title>{title}</title>")),
            "{path} の <title> が「{title}」でない: {html}"
        );
    }
}

#[test]
fn every_page_has_a_description_meta_derived_from_the_route() {
    for (path, description) in [
        (
            "/",
            "構造設計者向けの計算ツールと、数学的な可視化・書きもの。",
        ),
        ("/tools", "構造計算の入力・計算・判定。"),
        ("/studies", "数学やかたちを眺めて考えるための可視化。"),
        ("/notes", "構造や数学についての書きもの。"),
        ("/root", "root のための場所。"),
        ("/no/such/page", "この住所には何もない。"),
    ] {
        let html = render_path(path);
        let head = head_of(&html);
        assert!(
            head.contains(&format!(
                "<meta name=\"description\" content=\"{description}\""
            )),
            "{path} の description が「{description}」でない: {head}"
        );
    }
}

#[test]
fn head_links_the_fonts_and_the_stylesheet() {
    let html = render_path("/");
    let head = head_of(&html);
    assert!(
        head.contains("rel=\"stylesheet\""),
        "stylesheet が head に無い: {head}"
    );
    assert!(
        head.contains("fonts.googleapis.com"),
        "フォントの読込が head に無い: {head}"
    );
}

/// `needle` を含む最初の開きタグ（`<a href="/data" …>` など）。
fn tag_with<'a>(html: &'a str, needle: &str) -> &'a str {
    let at = html
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} が無い: {html}"));
    let start = html[..at].rfind('<').expect("タグの始まりが無い");
    let end = html[at..].find('>').expect("タグが閉じていない") + at;
    &html[start..=end]
}

#[test]
fn header_has_the_wordmark_then_the_four_sections_then_the_page_name_the_theme_and_the_account() {
    let html = render_path("/data/concrete");
    let header = element(&html, "header");
    assert_in_order(
        &text_of(header),
        &[
            "rustruct",
            ".matsumok.com",
            "Tools",
            "Data",
            "Studies",
            "Notes",
            "コンクリート",
        ],
    );
    assert!(
        header.contains("href=\"/\""),
        "wordmark からトップへ戻れない: {header}"
    );
    for path in ["/tools", "/data", "/studies", "/notes"] {
        assert!(
            header.contains(&format!("href=\"{path}\"")),
            "ヘッダーに {path} が無い: {header}"
        );
    }
    for label in ["メニューを開く", "ライトとダークを切り替える", "アカウント"]
    {
        assert!(
            header.contains(&format!("aria-label=\"{label}\"")),
            "ヘッダーに「{label}」のボタンが無い: {header}"
        );
    }
}

#[test]
fn the_header_comes_before_the_page_body() {
    let html = render_path("/tools");
    let header = html.find("<header").expect("header が無い");
    let main = html.find("<main").expect("main が無い");
    assert!(header < main, "ヘッダーが本文の後にある: {html}");
}

#[test]
fn home_has_no_page_name_in_the_header() {
    let html = render_path("/");
    let header = text_of(element(&html, "header"));
    assert!(
        !header.contains("Home"),
        "トップのヘッダーにページ名が出ている: {header}"
    );
}

#[test]
fn the_header_marks_the_section_of_the_page() {
    // 部のページそのものは page、その下のページは true（今いる部）
    for (path, current) in [
        ("/data", "aria-current=\"page\""),
        ("/data/concrete", "aria-current=\"true\""),
    ] {
        let html = render_path(path);
        let header = element(&html, "header");
        let link = tag_with(header, "href=\"/data\"");
        assert!(
            link.contains(current),
            "{path} で /data に {current} が無い: {link}"
        );
        assert_eq!(link.matches("aria-current").count(), 1, "印が二重: {link}");
        let other = tag_with(header, "href=\"/tools\"");
        assert!(
            !other.contains("aria-current"),
            "{path} で /tools に印がある: {other}"
        );
    }
    let home = render_path("/");
    let header = element(&home, "header");
    for path in ["/tools", "/data", "/studies", "/notes"] {
        let link = tag_with(header, &format!("href=\"{path}\""));
        assert!(
            !link.contains("aria-current"),
            "トップで {path} に印がある: {link}"
        );
    }
}

#[test]
fn head_applies_the_remembered_theme_before_the_page_is_drawn() {
    let html = render_path("/");
    let head = head_of(&html);
    let script = element(head, "script");
    for part in ["localStorage", "data-theme", "\"light\"", "\"dark\""] {
        assert!(
            script.contains(part),
            "head の script に {part} が無い: {script}"
        );
    }
}
