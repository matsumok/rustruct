//! シーン → SVG の文字列。色の役割名はここで [`Colors`] の書き方の色になる。

use std::fmt::Write;

use crate::Error;
use crate::color::blend;
use crate::scene::{Anchor, Item, Paint, Scene, Stroke};
use crate::theme::Theme;

/// 役割名の色の書き方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colors {
    /// テーマの値（`#rrggbb`）を `fill` / `stroke` の属性に書く。間の色は OKLab で混ぜた値。
    /// 色を読む描き手（wgpu）や、ページの外に出す静的な SVG のため
    Values,
    /// CSS 変数（`var(--color-ink)`）を `style` に書く。間の色は `color-mix(in oklab, …)`。
    /// 値はページの CSS が持ち、ライトとダークで同じ文字列になる
    Variables,
}

/// シーンを SVG に書く。役割名がテーマに無いか、帯の色の列の段数が使えなければ `Err`。
/// `Colors::Variables` でもテーマで役割名と段数を確かめ、書体はテーマの値を書く。
pub fn to_svg(scene: &Scene, theme: &Theme, colors: Colors) -> Result<String, Error> {
    let paints = Paints { theme, colors };
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
                    r#"<path d="{d}"{}/>"#,
                    paints.shape(fill.as_ref(), stroke.as_ref())?
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
                    r#"<rect x="{}" y="{}" width="{}" height="{}"{}/>"#,
                    num(*x),
                    num(*y),
                    num(*width),
                    num(*height),
                    paints.shape(fill.as_ref(), stroke.as_ref())?
                );
            }
            Item::Circle {
                center,
                radius,
                fill,
                stroke,
            } => {
                let _ = writeln!(
                    out,
                    r#"<circle cx="{}" cy="{}" r="{}"{}/>"#,
                    num(center[0]),
                    num(center[1]),
                    num(*radius),
                    paints.shape(fill.as_ref(), stroke.as_ref())?
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
                let color = paints.attrs(&[
                    ("fill", paints.paint(fill)?),
                    match halo {
                        Some(h) => ("stroke", paints.paint(&h.paint)?),
                        None => ("", String::new()),
                    },
                ]);
                let halo = match halo {
                    // 縁を文字の下に描く（paint-order）。縁の角は丸める
                    Some(h) => format!(
                        r#" stroke-width="{}" stroke-linejoin="round" paint-order="stroke""#,
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
                    r#"<text x="{}" y="{}" font-family="{}" font-size="{}"{color} text-anchor="{anchor}" dominant-baseline="central"{transform}{halo}>{}</text>"#,
                    num(*x),
                    num(*y),
                    escape(theme.font(*font)),
                    num(*size),
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

/// 役割名 → 色の書き方。`Colors` の 2 つの書き方の違いはここにだけある。
struct Paints<'a> {
    theme: &'a Theme,
    colors: Colors,
}

impl Paints<'_> {
    fn role(&self, role: &str) -> Result<String, Error> {
        let value = self
            .theme
            .color(role)
            .ok_or_else(|| Error::UnknownRole(role.to_string()))?;
        Ok(match self.colors {
            Colors::Values => value.to_string(),
            Colors::Variables => format!("var(--color-{role})"),
        })
    }

    fn paint(&self, p: &Paint) -> Result<String, Error> {
        match p {
            Paint::Role(role) => self.role(role),
            Paint::Blend { from, to, t } => match self.colors {
                Colors::Values => blend(&self.role(from)?, &self.role(to)?, *t),
                // 端は混ぜずにその色
                Colors::Variables if *t <= 0.0 => self.role(from),
                Colors::Variables if *t >= 1.0 => self.role(to),
                Colors::Variables => Ok(format!(
                    "color-mix(in oklab, {}, {} {}%)",
                    self.role(from)?,
                    self.role(to)?,
                    num(t * 100.0)
                )),
            },
            Paint::Band { palette, k, n } => self.paint(&palette.blend(*k, *n, self.theme)?),
        }
    }

    /// 色の組（`fill`、`stroke`）。値は属性に、CSS 変数は `style` にまとめて書く。名前が空の組は書かない
    fn attrs(&self, pairs: &[(&str, String)]) -> String {
        let pairs = pairs.iter().filter(|(name, _)| !name.is_empty());
        match self.colors {
            Colors::Values => pairs.map(|(name, v)| format!(r#" {name}="{v}""#)).collect(),
            Colors::Variables => {
                let style: Vec<String> = pairs.map(|(name, v)| format!("{name}: {v}")).collect();
                format!(r#" style="{}""#, style.join("; "))
            }
        }
    }

    /// パス・矩形・円の塗りと線。塗りが無ければ `none`
    fn shape(&self, fill: Option<&Paint>, stroke: Option<&Stroke>) -> Result<String, Error> {
        let fill = match fill {
            Some(p) => self.paint(p)?,
            None => "none".to_string(),
        };
        Ok(match stroke {
            Some(s) => format!(
                r#"{} stroke-width="{}" stroke-linejoin="round" stroke-linecap="round""#,
                self.attrs(&[("fill", fill), ("stroke", self.paint(&s.paint)?)]),
                num(s.width)
            ),
            None => self.attrs(&[("fill", fill)]),
        })
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

/// 継ぎ目: シーン → SVG の文字列。色はテーマの値か CSS 変数、文字は白抜き、未知の役割名は Err。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Palette;
    use crate::scene::{Font, Scene};
    use crate::theme::sample;

    fn line(stroke: Stroke) -> Item {
        Item::Path {
            subpaths: vec![vec![[0.0, 0.0], [10.0, 5.5]]],
            closed: false,
            fill: None,
            stroke: Some(stroke),
        }
    }

    fn fill(paint: Paint) -> Scene {
        let mut scene = Scene::new(10.0, 10.0);
        scene.items.push(Item::Path {
            subpaths: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]],
            closed: true,
            fill: Some(paint),
            stroke: None,
        });
        scene
    }

    fn values(scene: &Scene) -> String {
        to_svg(scene, &sample(), Colors::Values).unwrap()
    }

    fn variables(scene: &Scene) -> String {
        to_svg(scene, &sample(), Colors::Variables).unwrap()
    }

    #[test]
    fn roles_become_the_values_of_the_theme() {
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
        let svg = values(&scene);
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">"#));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(
            svg.contains(
                r##"<path d="M0 0L10 5.5" fill="none" stroke="#1c2024" stroke-width="0.25""##
            ),
            "{svg}"
        );
        assert!(svg.contains(r##"<rect x="1" y="2" width="3" height="4" fill="#f0f0f3"/>"##));
        // 別の値の表なら別の色
        let other = Theme::new([("ink", "#edeef0"), ("fill", "#000000")], "", "");
        let svg = to_svg(&scene, &other, Colors::Values).unwrap();
        assert!(svg.contains(r##"stroke="#edeef0""##), "{svg}");
    }

    #[test]
    fn roles_become_css_variables_in_the_style() {
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
        let svg = variables(&scene);
        assert!(
            svg.contains(r#"<path d="M0 0L10 5.5" style="fill: none; stroke: var(--color-ink)" stroke-width="0.25""#),
            "{svg}"
        );
        assert!(svg.contains(
            r#"<rect x="1" y="2" width="3" height="4" style="fill: var(--color-fill)"/>"#
        ));
        assert!(!svg.contains('#'), "{svg}");
        // 値の違う表でも同じ文字列
        let other = Theme::new([("ink", "#edeef0"), ("fill", "#000000")], "", "");
        assert_eq!(to_svg(&scene, &other, Colors::Variables).unwrap(), svg);
    }

    #[test]
    fn blend_paints_are_mixed_from_two_roles() {
        let at = |t| {
            fill(Paint::Blend {
                from: "seq-1".into(),
                to: "seq-2".into(),
                t,
            })
        };
        let svg = values(&at(0.0));
        assert!(svg.contains(r#"d="M0 0L1 0L1 1Z""#));
        assert!(svg.contains(&format!(r#"fill="{}""#, sample().color("seq-1").unwrap())));
        assert!(variables(&at(0.0)).contains(r#"style="fill: var(--color-seq-1)""#));
        assert!(variables(&at(1.0)).contains(r#"style="fill: var(--color-seq-2)""#));
        let svg = variables(&at(0.37));
        assert!(
            svg.contains(
                r#"style="fill: color-mix(in oklab, var(--color-seq-1), var(--color-seq-2) 37%)""#
            ),
            "{svg}"
        );
    }

    #[test]
    fn band_paints_take_the_stops_of_the_theme() {
        // 12 段の seq に 4 本の帯: 2 本目は seq-4 と seq-5 の間の 2/3
        let scene = fill(Palette::Seq.band_paint(1, 4));
        let svg = variables(&scene);
        assert!(
            svg.contains("color-mix(in oklab, var(--color-seq-4), var(--color-seq-5) 66.67%)"),
            "{svg}"
        );
        let svg = values(&scene);
        let mixed = blend(
            sample().color("seq-4").unwrap(),
            sample().color("seq-5").unwrap(),
            2.0 / 3.0,
        )
        .unwrap();
        assert!(svg.contains(&format!(r#"fill="{mixed}""#)), "{svg}");
        // 段が足りなければ Err
        let few = Theme::new([("seq-1", "#000000")], "", "");
        assert_eq!(
            to_svg(&scene, &few, Colors::Variables),
            Err(Error::Stops {
                prefix: "seq".into(),
                count: 1
            })
        );
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
        let svg = values(&scene);
        assert!(
            svg.contains(r#"font-family="&quot;Sample Mono&quot;, monospace""#),
            "{svg}"
        );
        assert!(svg.contains(r#"font-size="9""#));
        assert!(svg.contains(r##"fill="#1c2024" stroke="#ffffff" text-anchor="middle""##));
        assert!(svg.contains(r#"transform="rotate(-30 5 6)""#));
        assert!(svg.contains(r#"stroke-width="2.5" stroke-linejoin="round" paint-order="stroke""#));
        assert!(svg.contains(">a&lt;b &amp; &quot;c&quot;</text>"));
        assert!(variables(&scene).contains(
            r#"style="fill: var(--color-ink); stroke: var(--color-paper)" text-anchor="middle""#
        ));
    }

    #[test]
    fn circle_is_written_with_center_radius_fill_and_stroke() {
        let mut scene = Scene::new(10.0, 10.0);
        scene.items.push(Item::Circle {
            center: [4.0, 5.5],
            radius: 3.5,
            fill: Some(Paint::role("paper")),
            stroke: Some(Stroke::new("ink", 1.0)),
        });
        let svg = values(&scene);
        assert!(
            svg.contains(r##"<circle cx="4" cy="5.5" r="3.5" fill="#ffffff" stroke="#1c2024" stroke-width="1""##),
            "{svg}"
        );
    }

    #[test]
    fn unknown_role_is_an_error_in_both_ways() {
        let mut scene = Scene::new(10.0, 10.0);
        scene.items.push(line(Stroke::new("no-such-role", 1.0)));
        for colors in [Colors::Values, Colors::Variables] {
            assert_eq!(
                to_svg(&scene, &sample(), colors),
                Err(Error::UnknownRole("no-such-role".into()))
            );
        }
    }
}
