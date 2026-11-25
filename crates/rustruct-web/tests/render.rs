//! 継ぎ目 A（Issue #8 / #11）: 「パス → HTML 文字列」の関数 `render_path` を外から見る。
//! クラス名の並びは見ない。本文の文字と順序、どこで要素が分かれているか、`<title>` だけを見る。

mod common;
use common::markup::{count_tags, elements, tags, title};
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
            "構造設計で参照する材料と規格の値。",
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
            count_tags(&html, "a", &[("href", path)]) > 0,
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
    for (path, title) in [("/studies", "Studies"), ("/notes", "Notes")] {
        let html = render_path(path);
        let text = text_of(&html);
        // 枠の wordmark とパンくずの住所の後に、見出し、一文の順
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
    assert_eq!(count_tags(html, "h1", &[]), 1, "h1 が 1 つでない: {html}");
    text_of(element(html, "h1"))
}

/// 要素の中の最初の `<a>` の行き先。リンクが無ければ `None`
fn first_href(html: &str) -> Option<String> {
    tags(html)
        .iter()
        .find(|tag| tag.name == "a")
        .map(|a| a.get("href").expect("a に href が無い").to_string())
}

/// トップの一覧の各項目: (href, リンクの中の文字)。本文（`<main>`）の中のリンクだけ。
fn home_items(home: &str) -> Vec<(String, String)> {
    elements(element(home, "main"), "a")
        .into_iter()
        .map(|a| (first_href(a).expect("a に href が無い"), text_of(a)))
        .collect()
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
            "ページが見つからない。",
        ],
    );
    assert_eq!(h1_text(&html), "Not found");
    assert!(
        count_tags(&html, "a", &[("href", "/")]) > 0,
        "wordmark からトップへ戻れない: {html}"
    );
    assert!(
        !text.contains("まだ何もない"),
        "not-found が空ページの文言になっている: {text}"
    );
}

/// not-found は静的な 1 ファイル（`/404`）として書き出され、どの住所で開かれても同じ中身が届く。
/// 求められた住所はブラウザでしか決まらないので、初回描画には入れない（ADR site-0001 の規則）。
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
            "/tools/kirchhoff-plate",
            "/tools/kirchhoff-plate/manual",
            "/tools/slab-thickness",
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

/// `<head>` の要素。
fn head_of(html: &str) -> &str {
    element(html, "head")
}

/// `<head>` の description の meta の `content`。無ければ `None`
fn description_of(html: &str) -> Option<&str> {
    tags(head_of(html))
        .into_iter()
        .find(|tag| tag.name == "meta" && tag.get("name") == Some("description"))
        .and_then(|meta| meta.get("content"))
}

#[test]
fn title_is_page_name_then_site_name() {
    for (path, want) in [
        ("/", "Home · rustruct.matsumok.com"),
        ("/tools", "Tools · rustruct.matsumok.com"),
        (
            "/tools/kirchhoff-plate",
            "Kirchhoff 板の曲げ · rustruct.matsumok.com",
        ),
        (
            "/tools/kirchhoff-plate/manual",
            "Kirchhoff 板の曲げの説明書 · rustruct.matsumok.com",
        ),
        (
            "/tools/slab-thickness",
            "必要スラブ厚さ · rustruct.matsumok.com",
        ),
        ("/data", "Data · rustruct.matsumok.com"),
        ("/data/concrete", "コンクリート · rustruct.matsumok.com"),
        ("/data/rebar", "鉄筋 · rustruct.matsumok.com"),
        ("/studies", "Studies · rustruct.matsumok.com"),
        ("/notes", "Notes · rustruct.matsumok.com"),
        ("/root", "root · rustruct.matsumok.com"),
        ("/no/such/page", "Not found · rustruct.matsumok.com"),
    ] {
        let html = render_path(path);
        assert_eq!(
            title(head_of(&html)),
            Some(want),
            "{path} の <title> が「{want}」でない: {html}"
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
        ("/no/such/page", "ページが見つからない。"),
    ] {
        let html = render_path(path);
        assert_eq!(
            description_of(&html),
            Some(description),
            "{path} の description が「{description}」でない: {}",
            head_of(&html)
        );
    }
}

/// SSG が書き出す住所すべてに、空でない description がある（SSG の出力の検査 `check_ssg_output` は文言を見ない）。
#[test]
fn every_ssg_page_has_a_non_empty_description_meta() {
    for path in static_paths() {
        let html = render_path(&path);
        let description = description_of(&html)
            .unwrap_or_else(|| panic!("{path} に description が無い: {}", head_of(&html)));
        assert!(
            !description.is_empty(),
            "{path} の description が空: {}",
            head_of(&html)
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

#[test]
fn header_has_the_wordmark_the_address_the_theme_and_the_account_only() {
    let html = render_path("/data/concrete");
    let header = element(&html, "header");
    let text = text_of(header);
    assert_eq!(
        text, "rustruct.matsumok.com/data/concrete",
        "ヘッダーの文字はサイト名と住所だけ: {header}"
    );
    assert!(
        count_tags(header, "a", &[("href", "/")]) > 0,
        "wordmark からトップへ戻れない: {header}"
    );
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

/// ヘッダーのパンくず（`<nav>`）の各段: (href か None, 文字)。
fn breadcrumb(html: &str) -> Vec<(Option<String>, String)> {
    let nav = element(element(html, "header"), "nav");
    elements(nav, "li")
        .into_iter()
        .map(|li| (first_href(li), text_of(li)))
        .collect()
}

#[test]
fn breadcrumb_links_each_parent_and_leaves_the_current_page_unlinked() {
    let html = render_path("/data/concrete");
    assert_eq!(
        breadcrumb(&html),
        vec![
            (Some("/data".to_string()), "/data".to_string()),
            (None, "/concrete".to_string()),
        ]
    );
    assert!(
        element(element(&html, "header"), "nav").contains("aria-current=\"page\""),
        "今いる段に aria-current が無い"
    );
    assert_eq!(
        breadcrumb(&render_path("/tools")),
        vec![(None, "/tools".to_string())]
    );
    assert_eq!(
        breadcrumb(&render_path("/tools/kirchhoff-plate")),
        vec![
            (Some("/tools".to_string()), "/tools".to_string()),
            (None, "/kirchhoff-plate".to_string()),
        ]
    );
}

#[test]
fn home_and_not_found_have_no_breadcrumb() {
    // not-found はどの住所でも同じ 1 ファイルなので、住所を出せない（ADR site-0001）
    for path in ["/", "/tools/nothing"] {
        let html = render_path(path);
        assert!(
            !element(&html, "header").contains("<nav"),
            "{path} にパンくずがある"
        );
    }
}

#[test]
fn page_body_does_not_repeat_the_address() {
    // 住所はヘッダーのパンくず（iPad・PC）とメニューの木（スマホ）で示す。本文には出さない（#28）
    // 一覧のページ（/tools、/data）は下のページの住所を並べるので、ここでは見ない
    for path in [
        "/studies",
        "/tools/kirchhoff-plate",
        "/tools/kirchhoff-plate/manual",
        "/data/concrete",
        "/root",
    ] {
        let body = text_of(element(&render_path(path), "main"));
        assert!(!body.contains(path), "{path} の本文に住所がある: {body}");
    }
}

/// 本文（`<main>`）の見出しの次の兄弟（同じ親の中で見出しのすぐ後）に置いたリンク（見出しの右のバッジ）:
/// (href, 文字, aria-label)。見出しの閉じタグのすぐ後がリンクの開きタグでなければ `None`
fn badge_after_heading(html: &str) -> Option<(String, String, String)> {
    let main = element(html, "main");
    let after = &main[main.find("</h1>").expect("h1 が無い") + "</h1>".len()..];
    if !after.trim_start().starts_with("<a ") {
        return None;
    }
    let tag = tags(after).into_iter().next()?;
    let a = element(after, "a");
    Some((
        tag.get("href")?.to_string(),
        text_of(a),
        tag.get("aria-label")?.to_string(),
    ))
}

#[test]
fn the_tool_page_has_a_badge_to_its_manual_right_after_the_heading() {
    assert_eq!(
        badge_after_heading(&render_path("/tools/kirchhoff-plate")),
        Some((
            "/tools/kirchhoff-plate/manual".to_string(),
            "説明書".to_string(),
            "Kirchhoff 板の曲げの説明書".to_string(),
        ))
    );
    // 見出しは見出しの文字だけ（バッジは見出しの外）
    assert_eq!(
        h1_text(&render_path("/tools/kirchhoff-plate")),
        "Kirchhoff 板の曲げ"
    );
    // 説明書のないページには置かない
    for path in ["/tools/slab-thickness", "/tools/kirchhoff-plate/manual"] {
        assert_eq!(
            badge_after_heading(&render_path(path)),
            None,
            "{path} にバッジがある"
        );
    }
}

#[test]
fn the_manual_is_headed_and_reached_through_its_tool_in_the_breadcrumb() {
    let html = render_path("/tools/kirchhoff-plate/manual");
    assert_eq!(h1_text(&html), "Kirchhoff 板の曲げの説明書");
    assert_eq!(
        breadcrumb(&html),
        vec![
            (Some("/tools".to_string()), "/tools".to_string()),
            (
                Some("/tools/kirchhoff-plate".to_string()),
                "/kirchhoff-plate".to_string()
            ),
            (None, "/manual".to_string()),
        ]
    );
    assert!(
        !text_of(&render_path("/tools")).contains("/tools/kirchhoff-plate/manual"),
        "/tools の一覧に説明書がある"
    );
}

/// 説明書の式は描いた HTML の中で MathML に組まれていて、TeX の書き方は本文に残らない。
/// ブラウザで組む道具は読まない（読込むスクリプトは Tool のページと同じ）
#[test]
fn the_manual_formulas_are_typeset_in_the_html_without_more_scripts() {
    let html = render_path("/tools/kirchhoff-plate/manual");
    let article = element(element(&html, "main"), "article");
    let sections: Vec<String> = elements(&strip_annotations(article), "h2")
        .into_iter()
        .map(text_of)
        .collect();
    assert_eq!(
        sections,
        [
            "1 仮定と式",
            "2 符号と座標",
            "3 支持条件",
            "4 解き方と精度",
            "5 隅の扱いと最大値",
            "6 ν の扱い",
        ]
    );
    let headings: Vec<String> = elements(article, "h3").into_iter().map(text_of).collect();
    assert_eq!(
        headings[..3],
        ["仮定", "曲げ剛性とたわみの式", "結果に出る量"]
    );
    let display = count_tags(article, "math", &[("display", "block")]);
    assert!(display > 0, "別行の式が無い");
    assert!(count_tags(article, "math", &[]) > display, "文中の式が無い");
    let shown = text_of(&strip_annotations(article));
    for raw in ["$", "\\frac", "\\nabla"] {
        assert!(
            !shown.contains(raw),
            "本文に組んでいない {raw} が残っている: {shown}"
        );
    }
    assert!(!html.to_lowercase().contains("katex"), "KaTeX を読んでいる");
    assert_eq!(
        count_tags(&html, "script", &[]),
        count_tags(&render_path("/tools/kirchhoff-plate"), "script", &[]),
        "読込むスクリプトが Tool のページと違う"
    );
}

/// MathML の注釈（TeX の書き方を残す `annotation`）を外した HTML
fn strip_annotations(html: &str) -> String {
    elements(html, "annotation")
        .into_iter()
        .fold(html.to_string(), |html, a| html.replace(a, ""))
}
