//! シーン → SVG の文字列。色の役割名はここでテーマの値になる。

use std::fmt::Write;

use crate::Error;
use crate::color::blend;
use crate::scene::{Anchor, Item, Paint, Scene, Stroke};
use crate::theme::Theme;

/// シーンを SVG に書く。役割名が tokens.json に無ければ `Err`。
pub fn to_svg(scene: &Scene, theme: &Theme) -> Result<String, Error> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
        w = num(scene.width),
        h = num(scene.height)
    );
    for item in &scene.items {
        match item {
            Item::Path {
                subpaths,
                closed,
                fill,
                stroke,
            } => {
                let mut d = String::new();
                for subpath in subpaths {
                    for (k, p) in subpath.iter().enumerate() {
                        let _ = write!(
                            d,
                            "{}{} {}",
                            if k == 0 { "M" } else { "L" },
                            num(p[0]),
                            num(p[1])
                        );
                    }
                    if *closed && !subpath.is_empty() {
                        d.push('Z');
                    }
                }
                let _ = writeln!(
                    out,
                    r#"<path d="{d}" fill="{}"{}/>"#,
                    paint_or_none(theme, fill.as_ref())?,
                    stroke_attrs(theme, stroke.as_ref())?
                );
            }
            Item::Rect {
                x,
                y,
                width,
                height,
                fill,
                stroke,
            } => {
                let _ = writeln!(
                    out,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"{}/>"#,
                    num(*x),
                    num(*y),
                    num(*width),
                    num(*height),
                    paint_or_none(theme, fill.as_ref())?,
                    stroke_attrs(theme, stroke.as_ref())?
                );
            }
            Item::Text {
                x,
                y,
                text,
                size,
                font,
                fill,
                anchor,
                rotate,
                halo,
            } => {
                let anchor = match anchor {
                    Anchor::Start => "start",
                    Anchor::Middle => "middle",
                    Anchor::End => "end",
                };
                let halo = match halo {
                    // 縁を文字の下に描く（paint-order）。縁の角は丸める
                    Some(h) => format!(
                        r#" stroke="{}" stroke-width="{}" stroke-linejoin="round" paint-order="stroke""#,
                        paint(theme, &h.paint)?,
                        num(h.width)
                    ),
                    None => String::new(),
                };
                let transform = if *rotate == 0.0 {
                    String::new()
                } else {
                    format!(
                        r#" transform="rotate({} {} {})""#,
                        num(*rotate),
                        num(*x),
                        num(*y)
                    )
                };
                let _ = writeln!(
                    out,
                    r#"<text x="{}" y="{}" font-family="{}" font-size="{}" fill="{}" text-anchor="{anchor}" dominant-baseline="central"{transform}{halo}>{}</text>"#,
                    num(*x),
                    num(*y),
                    escape(theme.font(*font)),
                    num(*size),
                    paint(theme, fill)?,
                    escape(text)
                );
            }
        }
    }
    out.push_str("</svg>\n");
    Ok(out)
}

/// 座標と大きさの数。整数は整数のまま、それ以外は小数 2 桁まで。
fn num(v: f64) -> String {
    if v == v.round() {
        format!("{}", v as i64)
    } else {
        let s = format!("{v:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn paint(theme: &Theme, p: &Paint) -> Result<String, Error> {
    let color = |role: &str| {
        theme
            .color(role)
            .ok_or_else(|| Error::UnknownRole(role.to_string()))
    };
    match p {
        Paint::Role(role) => Ok(color(role)?.to_string()),
        Paint::Blend { from, to, t } => blend(color(from)?, color(to)?, *t),
    }
}

fn paint_or_none(theme: &Theme, p: Option<&Paint>) -> Result<String, Error> {
    match p {
        Some(p) => paint(theme, p),
        None => Ok("none".to_string()),
    }
}

fn stroke_attrs(theme: &Theme, s: Option<&Stroke>) -> Result<String, Error> {
    match s {
        Some(s) => Ok(format!(
            r#" stroke="{}" stroke-width="{}" stroke-linejoin="round" stroke-linecap="round""#,
            paint(theme, &s.paint)?,
            num(s.width)
        )),
        None => Ok(String::new()),
    }
}

/// 文字と属性値の `& < > "` を実体参照にする。
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// 継ぎ目: シーン → SVG の文字列。色は tokens.json の値、文字は白抜き、未知の役割名は Err。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Font, Scene};

    fn line(stroke: Stroke) -> Item {
        Item::Path {
            subpaths: vec![vec![[0.0, 0.0], [10.0, 5.5]]],
            closed: false,
            fill: None,
            stroke: Some(stroke),
        }
    }

    #[test]
    fn roles_become_token_colors_of_the_theme() {
        let mut scene = Scene::new(100.0, 50.0);
        scene.items.push(line(Stroke::new("ink", 0.25)));
        scene.items.push(Item::Rect {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
            fill: Some(Paint::role("fill")),
            stroke: None,
        });
        let light = to_svg(&scene, &Theme::light()).unwrap();
        let dark = to_svg(&scene, &Theme::dark()).unwrap();
        assert!(light.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">"#));
        assert!(light.trim_end().ends_with("</svg>"));
        assert!(
            light.contains(
                r##"<path d="M0 0L10 5.5" fill="none" stroke="#1c2024" stroke-width="0.25""##
            ),
            "{light}"
        );
        assert!(light.contains(&format!(
            r#"fill="{}""#,
            Theme::light().color("fill").unwrap()
        )));
        assert!(dark.contains(&format!(
            r#"stroke="{}""#,
            Theme::dark().color("ink").unwrap()
        )));
        assert_ne!(light, dark);
    }

    #[test]
    fn blend_paints_are_mixed_from_two_roles() {
        let mut scene = Scene::new(10.0, 10.0);
        scene.items.push(Item::Path {
            subpaths: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]],
            closed: true,
            fill: Some(Paint::Blend {
                from: "seq-1".into(),
                to: "seq-2".into(),
                t: 0.0,
            }),
            stroke: None,
        });
        let svg = to_svg(&scene, &Theme::light()).unwrap();
        assert!(svg.contains(r#"d="M0 0L1 0L1 1Z""#));
        assert!(svg.contains(&format!(
            r#"fill="{}""#,
            Theme::light().color("seq-1").unwrap()
        )));
    }

    #[test]
    fn text_is_outlined_rotated_and_escaped() {
        let mut scene = Scene::new(10.0, 10.0);
        scene.items.push(Item::Text {
            x: 5.0,
            y: 6.0,
            text: "a<b & \"c\"".into(),
            size: 9.0,
            font: Font::Mono,
            fill: Paint::role("ink"),
            anchor: Anchor::Middle,
            rotate: -30.0,
            halo: Some(Stroke::new("paper", 2.5)),
        });
        let svg = to_svg(&scene, &Theme::light()).unwrap();
        assert!(
            svg.contains(r#"font-family="&quot;IBM Plex Mono&quot;"#),
            "{svg}"
        );
        assert!(svg.contains(r#"font-size="9""#));
        assert!(svg.contains(r#"text-anchor="middle""#));
        assert!(svg.contains(r#"transform="rotate(-30 5 6)""#));
        assert!(svg.contains(
            r##"stroke="#ffffff" stroke-width="2.5" stroke-linejoin="round" paint-order="stroke""##
        ));
        assert!(svg.contains(">a&lt;b &amp; &quot;c&quot;</text>"));
    }

    #[test]
    fn unknown_role_is_an_error() {
        let mut scene = Scene::new(10.0, 10.0);
        scene.items.push(line(Stroke::new("no-such-role", 1.0)));
        assert_eq!(
            to_svg(&scene, &Theme::light()),
            Err(Error::UnknownRole("no-such-role".into()))
        );
    }
}
