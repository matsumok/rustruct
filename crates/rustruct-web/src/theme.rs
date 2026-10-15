//! `docs/design/tokens.json` → Tailwind v4 の `@theme` ブロックと、Google Fonts の読込 URL。
//!
//! 継ぎ目 B の純粋関数。I/O を持たず、文字列を受けて文字列を返す。`build.rs` がこれを
//! `#[path]` で取り込んで `theme.css` と `fonts.rs` を書き、`cargo test` が同じ関数を
//! tokens.json で検査する。ブラウザにも配信物にも入らない。

use std::fmt;

use serde_json::Value;

/// tokens.json を変換できなかった理由。
#[derive(Debug)]
pub enum Error {
    /// JSON として読めない。
    Json(serde_json::Error),
    /// 期待した形になっていない（キーが無い、型が違う、名前の接頭辞が違う）。
    Shape(String),
    /// `{ink}` のような参照の先が無い、または参照が循環している。
    Reference(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Json(e) => write!(f, "tokens.json を JSON として読めない: {e}"),
            Error::Shape(what) => write!(f, "tokens.json の形が違う: {what}"),
            Error::Reference(what) => write!(f, "tokens.json の参照を解決できない: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

/// tokens.json の文字列から、Tailwind v4 の `@theme` ブロックを含む CSS を作る。
///
/// 出し方（Tailwind の名前空間 → マークアップのクラス）:
/// - 色 `--color-<役割>`: `bg-paper` `text-ink` `border-line`
/// - 文字 `--text-<役割>` と `--line-height` `--font-weight` `--letter-spacing` の組: `text-body`
/// - 書体 `--font-sans` `--font-mono`: `font-sans` `font-mono`
/// - 余白 `--spacing-<n>`（tokens の `space-<n>`）: `p-5` `gap-4`
/// - 角丸 `--radius-<n>`（tokens の `radius-<n>`）: `rounded-0` `rounded-8` `rounded-full`
/// - 幅 `--container-<n>`（tokens の `layout` の `width-<n>`）: `max-w-column` `max-w-page`
/// - 切り替わる幅 `--breakpoint-<n>`（tokens の `breakpoint-<n>`）: `md:flex` `lg:px-8`
///
/// 各名前空間の Tailwind 既定値は消すので、tokens.json にない色や大きさのクラスや、`sm:` のような
/// 決めていない切り替わる幅は存在しない。文字の `md` の値は、768px から同じ役割名を `:root` で上書きする。
/// ダークは `<html>` の `data-theme`（ページで選んだ色）を先に見て、無ければ OS 設定に従う。
/// 同じ役割名の色を `:root` で上書きする。
pub fn theme_css(tokens_json: &str) -> Result<String, Error> {
    let tokens = Tokens::parse(tokens_json)?;
    let mut out = String::new();

    out.push_str(
        "/* docs/design/tokens.json から生成した Tailwind v4 の @theme。手で編集しない。\n\
         \x20  生成: crates/rustruct-web/build.rs（tokens.json を変えると cargo のビルドで更新される） */\n\n",
    );
    out.push_str(
        "/* ダーク: ページで選んだ色（<html data-theme>）が先、選んでいなければ OS 設定 */\n\
         @custom-variant dark {\n\
         \x20 &:where([data-theme=\"dark\"], [data-theme=\"dark\"] *) {\n    @slot;\n  }\n\
         \x20 @media (prefers-color-scheme: dark) {\n\
         \x20   &:where(:not([data-theme=\"light\"], [data-theme=\"light\"] *)) {\n      @slot;\n    }\n  }\n\
         }\n\n",
    );

    out.push_str("@theme {\n");

    out.push_str("  /* 色: 役割名。bg-paper / text-ink / border-line のように使う */\n");
    out.push_str("  --color-*: initial;\n");
    for c in &tokens.colors {
        out.push_str(&format!("  --color-{}: {};\n", c.name, c.light));
    }

    out.push_str("\n  /* 書体: font-sans / font-mono */\n");
    out.push_str("  --font-serif: initial;\n");
    for f in &tokens.families {
        out.push_str(&format!("  --font-{}: {};\n", f.name, f.stack));
    }

    out.push_str(
        "\n  /* 文字: 役割ごとに大きさ・行送り・太さ・字間の組。text-body のように使う */\n",
    );
    out.push_str("  --text-*: initial;\n");
    for s in &tokens.text_styles {
        out.push_str(&format!("  --text-{}: {};\n", s.name, s.font_size));
        out.push_str(&format!(
            "  --text-{}--line-height: {};\n",
            s.name, s.line_height
        ));
        out.push_str(&format!(
            "  --text-{}--font-weight: {};\n",
            s.name, s.font_weight
        ));
        if let Some(tracking) = &s.letter_spacing {
            out.push_str(&format!(
                "  --text-{}--letter-spacing: {tracking};\n",
                s.name
            ));
        }
    }

    out.push_str("\n  /* 余白: space-<n> を p-<n> / gap-<n> / m-<n> で使う */\n");
    out.push_str("  --spacing: initial;\n");
    for (name, value) in &tokens.spacing {
        out.push_str(&format!("  --spacing-{name}: {value};\n"));
    }

    out.push_str("\n  /* 角丸: radius-<n> を rounded-<n> で使う */\n");
    out.push_str("  --radius-*: initial;\n");
    for (name, value) in &tokens.radius {
        out.push_str(&format!("  --radius-{name}: {value};\n"));
    }

    out.push_str("\n  /* 幅: width-<n> を max-w-<n> / w-<n> で使う */\n");
    out.push_str("  --container-*: initial;\n");
    for (name, value) in &tokens.widths {
        out.push_str(&format!("  --container-{name}: {value};\n"));
    }

    out.push_str("\n  /* 切り替わる幅: breakpoint-<n> を <n>: で使う。決めた幅だけ */\n");
    out.push_str("  --breakpoint-*: initial;\n");
    for (name, value) in &tokens.breakpoints {
        out.push_str(&format!("  --breakpoint-{name}: {value};\n"));
    }

    out.push_str("}\n\n");

    out.push_str("/* 768px から: 大きな文字と Notes の本文だけ大きくする（docs/adr/0007） */\n");
    out.push_str(":root {\n  @variant md {\n");
    for s in &tokens.text_styles {
        if let Some((size, line_height)) = &s.md {
            out.push_str(&format!("    --text-{}: {size};\n", s.name));
            out.push_str(&format!(
                "    --text-{}--line-height: {line_height};\n",
                s.name
            ));
        }
    }
    out.push_str("  }\n}\n\n");

    out.push_str("/* ダーク: 同じ役割名の色を切り替える。フォームとスクロールバーも color-scheme で合わせる */\n");
    out.push_str(":root {\n  color-scheme: light;\n  @variant dark {\n    color-scheme: dark;\n");
    for c in &tokens.colors {
        out.push_str(&format!("    --color-{}: {};\n", c.name, c.dark));
    }
    out.push_str("  }\n}\n");

    Ok(out)
}

/// tokens.json の文字列から、Google Fonts CSS の URL を作る。
///
/// 書体は families の先頭の名前（引用符つきの最初の一つ）、太さは各書体を使う文字スタイルの
/// `fontWeight` の集合。文字スタイルを足せば必要な太さが自動で増え、使わない太さは読まない。
pub fn google_fonts_url(tokens_json: &str) -> Result<String, Error> {
    let tokens = Tokens::parse(tokens_json)?;
    let mut params = Vec::new();
    for f in &tokens.families {
        let primary = f
            .stack
            .strip_prefix('"')
            .and_then(|s| s.split_once('"'))
            .map(|(name, _)| name)
            .ok_or_else(|| {
                Error::Shape(format!(
                    "families/{} の先頭が引用符つきの書体名でない: {}",
                    f.name, f.stack
                ))
            })?;
        let mut weights: Vec<u64> = tokens
            .text_styles
            .iter()
            .filter(|s| s.family == f.name)
            .map(|s| s.font_weight)
            .collect();
        weights.sort_unstable();
        weights.dedup();
        if weights.is_empty() {
            continue;
        }
        let weights: Vec<String> = weights.iter().map(u64::to_string).collect();
        params.push(format!(
            "family={}:wght@{}",
            primary.replace(' ', "+"),
            weights.join(";")
        ));
    }
    Ok(format!(
        "https://fonts.googleapis.com/css2?{}&display=swap",
        params.join("&")
    ))
}

/// tokens.json のうち、ここで使う部分。参照は解決済み。
struct Tokens {
    colors: Vec<Color>,
    families: Vec<Family>,
    text_styles: Vec<TextStyle>,
    /// `space-<n>` の `<n>` と値。
    spacing: Vec<(String, String)>,
    /// `radius-<n>` の `<n>` と値。
    radius: Vec<(String, String)>,
    /// `layout` の `width-<n>` の `<n>` と値。
    widths: Vec<(String, String)>,
    /// `breakpoint` の `breakpoint-<n>` の `<n>` と値。
    breakpoints: Vec<(String, String)>,
}

/// 色の役割名と、ライト・ダークそれぞれの値。
struct Color {
    name: String,
    light: String,
    dark: String,
}

/// 書体の役割名（`sans` / `mono`）とフォールバック込みの font-family。
struct Family {
    name: String,
    stack: String,
}

/// 文字の役割と、その大きさ・行送り・太さ・字間。
struct TextStyle {
    name: String,
    family: String,
    font_size: String,
    line_height: String,
    font_weight: u64,
    letter_spacing: Option<String>,
    /// 768px からの大きさと行送り。画面幅で変えない役割には無い。
    md: Option<(String, String)>,
}

impl Tokens {
    fn parse(tokens_json: &str) -> Result<Self, Error> {
        let v: Value = serde_json::from_str(tokens_json)?;
        Ok(Tokens {
            colors: colors(&v)?,
            families: families(&v)?,
            text_styles: text_styles(&v)?,
            spacing: scale(&v, "/spacing/tokens", "space-")?,
            radius: scale(&v, "/radius/tokens", "radius-")?,
            widths: scale(&v, "/layout/tokens", "width-")?,
            breakpoints: scale(&v, "/breakpoint/tokens", "breakpoint-")?,
        })
    }
}

fn colors(v: &Value) -> Result<Vec<Color>, Error> {
    let raw: Vec<Color> = array(v, "/color/tokens")?
        .iter()
        .map(|t| {
            Ok(Color {
                name: str_at(t, "/name")?.to_string(),
                light: str_at(t, "/value/light")?.to_string(),
                dark: str_at(t, "/value/dark")?.to_string(),
            })
        })
        .collect::<Result<_, Error>>()?;

    raw.iter()
        .map(|c| {
            Ok(Color {
                name: c.name.clone(),
                light: resolve(&c.light, |n| {
                    raw.iter().find(|o| o.name == n).map(|o| &o.light)
                })?,
                dark: resolve(&c.dark, |n| {
                    raw.iter().find(|o| o.name == n).map(|o| &o.dark)
                })?,
            })
        })
        .collect()
}

/// `{ink}` のような参照を、同じテーマの値で置き換える。参照の先が参照でもよい。
fn resolve<'a>(
    value: &'a str,
    lookup: impl Fn(&str) -> Option<&'a String>,
) -> Result<String, Error> {
    let mut current = value;
    let mut visited: Vec<&str> = Vec::new();
    while let Some(name) = current.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        if visited.contains(&name) {
            return Err(Error::Reference(format!("{value} の参照が循環している")));
        }
        visited.push(name);
        current = lookup(name)
            .ok_or_else(|| Error::Reference(format!("{value} の参照先 {name} が無い")))?;
    }
    Ok(current.to_string())
}

fn families(v: &Value) -> Result<Vec<Family>, Error> {
    at(v, "/type/families")?
        .as_object()
        .ok_or_else(|| Error::Shape("/type/families がオブジェクトでない".into()))?
        .iter()
        .map(|(name, stack)| {
            Ok(Family {
                name: name.clone(),
                stack: stack
                    .as_str()
                    .ok_or_else(|| Error::Shape(format!("/type/families/{name} が文字列でない")))?
                    .to_string(),
            })
        })
        .collect()
}

fn text_styles(v: &Value) -> Result<Vec<TextStyle>, Error> {
    let mut styles = Vec::new();
    for group in array(v, "/type/groups")? {
        let family = str_at(group, "/family")?;
        for s in array(group, "/styles")? {
            let name = str_at(s, "/name")?;
            styles.push(TextStyle {
                name: name.to_string(),
                family: family.to_string(),
                font_size: str_at(s, "/fontSize")?.to_string(),
                line_height: str_at(s, "/lineHeight")?.to_string(),
                font_weight: at(s, "/fontWeight")?
                    .as_u64()
                    .ok_or_else(|| Error::Shape(format!("{name} の fontWeight が整数でない")))?,
                letter_spacing: s
                    .pointer("/letterSpacing")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                md: match s.pointer("/md") {
                    None => None,
                    Some(md) => Some((
                        str_at(md, "/fontSize")?.to_string(),
                        str_at(md, "/lineHeight")?.to_string(),
                    )),
                },
            });
        }
    }
    Ok(styles)
}

/// `<prefix><n>` という名前と値の並び（余白・角丸・幅）。接頭辞を外した `<n>` を返す。
fn scale(v: &Value, pointer: &str, prefix: &str) -> Result<Vec<(String, String)>, Error> {
    array(v, pointer)?
        .iter()
        .map(|t| {
            let name = str_at(t, "/name")?;
            let n = name.strip_prefix(prefix).ok_or_else(|| {
                Error::Shape(format!("{pointer} の {name} が {prefix} で始まっていない"))
            })?;
            Ok((n.to_string(), str_at(t, "/value")?.to_string()))
        })
        .collect()
}

fn at<'a>(v: &'a Value, pointer: &str) -> Result<&'a Value, Error> {
    v.pointer(pointer)
        .ok_or_else(|| Error::Shape(format!("{pointer} が無い")))
}

fn array<'a>(v: &'a Value, pointer: &str) -> Result<&'a [Value], Error> {
    at(v, pointer)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Shape(format!("{pointer} が配列でない")))
}

fn str_at<'a>(v: &'a Value, pointer: &str) -> Result<&'a str, Error> {
    at(v, pointer)?
        .as_str()
        .ok_or_else(|| Error::Shape(format!("{pointer} が文字列でない")))
}

/// Issue #10 の継ぎ目 B: tokens.json の文字列 → `@theme` 文字列。
/// `docs/design/tokens.json` そのものを固定資料として読む。期待値は tokens.json に書いてある値。
#[cfg(test)]
mod tests {
    use super::{google_fonts_url, theme_css};
    use serde_json::Value;

    const TOKENS: &str = include_str!("../../../docs/design/tokens.json");

    fn css() -> String {
        theme_css(TOKENS).expect("tokens.json から @theme を生成できる")
    }

    fn tokens() -> Value {
        serde_json::from_str(TOKENS).unwrap()
    }

    /// `@variant dark { ... }` の中身。@theme の外にあって、同じ役割名を上書きする。
    fn dark_block(css: &str) -> &str {
        let start = css.find("@variant dark {").expect("@variant dark が無い");
        let rest = &css[start..];
        let end = rest.find("\n  }").expect("@variant dark が閉じていない");
        &rest[..end]
    }

    #[test]
    fn every_color_token_comes_out_under_its_role_name_with_its_light_and_dark_values() {
        let css = css();
        let dark = dark_block(&css);
        let colors = tokens()["color"]["tokens"].as_array().unwrap().clone();
        assert!(colors.len() >= 39, "tokens.json の色が減っている");
        for t in colors {
            let name = t["name"].as_str().unwrap();
            let light = t["value"]["light"].as_str().unwrap();
            let dark_value = t["value"]["dark"].as_str().unwrap();
            if light.starts_with('{') {
                continue; // 参照は次のテストで見る
            }
            let line = format!("--color-{name}: {light};");
            assert!(css.contains(&line), "{line} が無い:\n{css}");
            let line = format!("--color-{name}: {dark_value};");
            assert!(dark.contains(&line), "ダーク側に {line} が無い:\n{dark}");
        }
        // 代表値をリテラルで
        assert!(css.contains("--color-paper: #ffffff;"));
        assert!(css.contains("--color-muted: #60646c;"));
        assert!(dark.contains("--color-paper: #111113;"));
        assert!(dark.contains("--color-ink: #edeef0;"));
        assert!(dark.contains("--color-muted: #b0b4ba;"));
    }

    #[test]
    fn a_reference_like_ink_is_resolved_to_the_referenced_value_on_both_themes() {
        let css = css();
        assert!(
            css.contains("--color-chart-3: #1c2024;"),
            "ライトの chart-3 が ink の値でない:\n{css}"
        );
        assert!(!css.contains("{ink}"), "参照が残っている:\n{css}");
        let dark = dark_block(&css);
        assert!(
            dark.contains("--color-chart-3: #edeef0;"),
            "ダークの chart-3 が ink のダーク値でない:\n{dark}"
        );
    }

    #[test]
    fn dark_follows_the_os_setting_unless_the_page_chose_a_theme() {
        let css = css();
        let variant = css
            .find("@custom-variant dark {")
            .map(|at| {
                &css[at..at
                    + css[at..]
                        .find("\n}\n")
                        .expect("@custom-variant が閉じていない")]
            })
            .unwrap_or_else(|| panic!("@custom-variant dark が無い:\n{css}"));
        // 選んだダークは OS に依らず、OS のダークは選んだライトに負ける（ui-decisions「画面幅」の切り替え）
        for part in [
            r#"&:where([data-theme="dark"], [data-theme="dark"] *)"#,
            "@media (prefers-color-scheme: dark)",
            r#"&:where(:not([data-theme="light"], [data-theme="light"] *))"#,
        ] {
            assert!(variant.contains(part), "{part} が無い:\n{variant}");
        }
    }

    #[test]
    fn form_controls_and_scrollbars_follow_the_theme_through_color_scheme() {
        let css = css();
        assert!(css.contains("color-scheme: light;"), "{css}");
        assert!(dark_block(&css).contains("color-scheme: dark;"), "{css}");
    }

    #[test]
    fn only_the_two_breakpoints_in_tokens_exist() {
        let css = css();
        for line in [
            "--breakpoint-*: initial;",
            "--breakpoint-md: 768px;",
            "--breakpoint-lg: 1024px;",
        ] {
            assert!(css.contains(line), "{line} が無い:\n{css}");
        }
    }

    /// `@variant md { ... }` の中身。768px から文字の大きさを上書きする。
    fn md_block(css: &str) -> &str {
        let start = css.find("@variant md {").expect("@variant md が無い");
        let rest = &css[start..];
        let end = rest.find("\n  }").expect("@variant md が閉じていない");
        &rest[..end]
    }

    #[test]
    fn large_text_and_prose_grow_from_the_md_breakpoint_and_nothing_else_does() {
        let css = css();
        let md = md_block(&css);
        for line in [
            "--text-display: 36px;",
            "--text-display--line-height: 48px;",
            "--text-title: 26px;",
            "--text-title--line-height: 36px;",
            "--text-figure: 64px;",
            "--text-figure--line-height: 64px;",
            "--text-prose: 17px;",
            "--text-prose--line-height: 32px;",
        ] {
            assert!(md.contains(line), "768px からの {line} が無い:\n{md}");
        }
        for fixed in [
            "--text-body:",
            "--text-heading:",
            "--text-label:",
            "--text-num:",
        ] {
            assert!(!md.contains(fixed), "{fixed} は画面幅で変えない:\n{md}");
        }
    }

    #[test]
    fn every_text_role_is_a_set_of_size_line_height_weight_and_letter_spacing() {
        let css = css();
        for group in tokens()["type"]["groups"].as_array().unwrap() {
            for s in group["styles"].as_array().unwrap() {
                let name = s["name"].as_str().unwrap();
                for line in [
                    format!("--text-{name}: {};", s["fontSize"].as_str().unwrap()),
                    format!(
                        "--text-{name}--line-height: {};",
                        s["lineHeight"].as_str().unwrap()
                    ),
                    format!("--text-{name}--font-weight: {};", s["fontWeight"]),
                ] {
                    assert!(css.contains(&line), "{line} が無い:\n{css}");
                }
                let tracking = format!("--text-{name}--letter-spacing");
                assert_eq!(
                    css.contains(&tracking),
                    s.get("letterSpacing").is_some(),
                    "{name} の字間の有無が tokens.json と違う:\n{css}"
                );
            }
        }
        // 代表値をリテラルで。wordmark は等幅の 12px / 500、display だけ字間を持つ
        for line in [
            "--text-wordmark: 12px;",
            "--text-wordmark--line-height: 16px;",
            "--text-wordmark--font-weight: 500;",
            "--text-display: 30px;",
            "--text-display--letter-spacing: -0.01em;",
            "--text-figure: 56px;",
        ] {
            assert!(css.contains(line), "{line} が無い:\n{css}");
        }
    }

    #[test]
    fn font_families_come_from_tokens_with_their_fallbacks() {
        let css = css();
        assert!(
            css.contains(
                "--font-sans: \"IBM Plex Sans JP\", \"Hiragino Sans\", \"BIZ UDPGothic\", sans-serif;"
            ),
            "sans が families どおりでない:\n{css}"
        );
        assert!(
            css.contains(
                "--font-mono: \"IBM Plex Mono\", \"SF Mono\", Menlo, Consolas, monospace;"
            ),
            "mono が families どおりでない:\n{css}"
        );
    }

    #[test]
    fn spacing_and_radius_tokens_are_usable_under_tailwinds_namespaces() {
        let css = css();
        for line in [
            "--spacing-0: 0px;",
            "--spacing-1: 4px;",
            "--spacing-2: 8px;",
            "--spacing-3: 12px;",
            "--spacing-4: 16px;",
            "--spacing-5: 20px;",
            "--spacing-6: 28px;",
            "--spacing-7: 40px;",
            "--spacing-8: 48px;",
            "--radius-0: 0px;",
            "--radius-8: 8px;",
            "--radius-full: 9999px;",
        ] {
            assert!(css.contains(line), "{line} が無い:\n{css}");
        }
    }

    #[test]
    fn layout_widths_are_usable_as_tailwind_container_sizes() {
        // 本文の1単位（ui-decisions「列の単位」）。max-w-column / w-column で使う
        assert!(css().contains("--container-column: 335px;"), "{}", css());
        // 本文の箱の最大幅（2単位と溝と区切り。docs/adr/0007 の 2u + 16 + 17）。max-w-page で使う
        assert!(css().contains("--container-page: 703px;"), "{}", css());
    }

    #[test]
    fn tailwind_defaults_for_the_token_namespaces_are_cleared_so_only_role_names_exist() {
        let css = css();
        for line in [
            "--color-*: initial;",
            "--text-*: initial;",
            "--spacing: initial;",
            "--radius-*: initial;",
        ] {
            assert!(css.contains(line), "{line} が無い:\n{css}");
        }
    }

    #[test]
    fn google_fonts_url_loads_both_plex_families_in_the_weights_the_styles_use() {
        assert_eq!(
            google_fonts_url(TOKENS).unwrap(),
            "https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@300;400;500&family=IBM+Plex+Sans+JP:wght@300;400;500&display=swap"
        );
    }

    #[test]
    fn broken_json_is_an_error_not_a_panic() {
        assert!(theme_css("{ not json").is_err());
    }

    #[test]
    fn a_dangling_reference_is_an_error() {
        let json = r##"{"color":{"tokens":[{"name":"a","value":{"light":"{nope}","dark":"#000"}}]},
            "type":{"families":{},"groups":[]},"spacing":{"tokens":[]},"radius":{"tokens":[]},"layout":{"tokens":[]},"breakpoint":{"tokens":[]}}"##;
        let err = theme_css(json).expect_err("参照先が無いのに通った");
        assert!(err.to_string().contains("nope"), "{err}");
    }

    #[test]
    fn a_reference_cycle_is_an_error() {
        let json = r##"{"color":{"tokens":[
            {"name":"a","value":{"light":"{b}","dark":"#000"}},
            {"name":"b","value":{"light":"{a}","dark":"#000"}}]},
            "type":{"families":{},"groups":[]},"spacing":{"tokens":[]},"radius":{"tokens":[]},"layout":{"tokens":[]},"breakpoint":{"tokens":[]}}"##;
        let err = theme_css(json).expect_err("循環しているのに通った");
        assert!(err.to_string().contains("循環"), "{err}");
    }
}
