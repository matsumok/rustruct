//! パス・文字・矩形だけの小さなシーンと、SVG の描き手。色は役割名で持ち、描くときにテーマで引く。

use std::collections::HashMap;
use std::fmt::Write;

use crate::contour::Pt;

pub type Role = String;

pub struct Stroke {
    pub role: Role,
    pub width: f64,
}

pub enum Font {
    Mono,
    Sans,
}

pub enum Item {
    Path { subpaths: Vec<Vec<Pt>>, closed: bool, fill: Option<Role>, stroke: Option<Stroke> },
    Rect { x: f64, y: f64, w: f64, h: f64, fill: Option<Role>, stroke: Option<Stroke> },
    Text {
        x: f64,
        y: f64,
        s: String,
        size: f64,
        font: Font,
        role: Role,
        anchor: &'static str,
        rotate: f64,
        /// 白抜き：文字の周りを地の色で縁取って下の線と塗りを消す。
        halo: Option<Stroke>,
    },
}

pub struct Scene {
    pub w: f64,
    pub h: f64,
    pub items: Vec<Item>,
}

pub type Theme = HashMap<String, String>;

fn paint(theme: &Theme, role: &Option<Role>) -> String {
    match role {
        Some(r) => theme.get(r).unwrap_or_else(|| panic!("no role {r}")).clone(),
        None => "none".into(),
    }
}

fn stroke_attrs(theme: &Theme, s: &Option<Stroke>) -> String {
    match s {
        Some(s) => format!(
            r#" stroke="{}" stroke-width="{}" stroke-linejoin="round" stroke-linecap="round""#,
            theme[&s.role], s.width
        ),
        None => String::new(),
    }
}

pub fn to_svg(scene: &Scene, theme: &Theme) -> String {
    let mut o = String::new();
    writeln!(
        o,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
        w = scene.w,
        h = scene.h
    )
    .unwrap();
    writeln!(o, r#"<rect width="100%" height="100%" fill="{}"/>"#, theme["paper"]).unwrap();
    for item in &scene.items {
        match item {
            Item::Path { subpaths, closed, fill, stroke } => {
                let mut d = String::new();
                for sp in subpaths {
                    for (k, p) in sp.iter().enumerate() {
                        write!(d, "{}{:.2} {:.2}", if k == 0 { "M" } else { "L" }, p[0], p[1]).unwrap();
                    }
                    if *closed {
                        d.push('Z');
                    }
                }
                writeln!(o, r#"<path d="{d}" fill="{}"{}/>"#, paint(theme, fill), stroke_attrs(theme, stroke))
                    .unwrap();
            }
            Item::Rect { x, y, w, h, fill, stroke } => {
                writeln!(
                    o,
                    r#"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" fill="{}"{}/>"#,
                    paint(theme, fill),
                    stroke_attrs(theme, stroke)
                )
                .unwrap();
            }
            Item::Text { x, y, s, size, font, role, anchor, rotate, halo } => {
                let family = match font {
                    Font::Mono => "IBM Plex Mono",
                    Font::Sans => "IBM Plex Sans JP",
                };
                let halo = match halo {
                    Some(h) => format!(
                        r#" stroke="{}" stroke-width="{}" stroke-linejoin="round" paint-order="stroke""#,
                        theme[&h.role], h.width
                    ),
                    None => String::new(),
                };
                writeln!(
                    o,
                    r#"<text x="{x:.2}" y="{y:.2}" font-family="{family}" font-size="{size}" fill="{}" text-anchor="{anchor}" dominant-baseline="central" transform="rotate({rotate:.2} {x:.2} {y:.2})"{halo}>{s}</text>"#,
                    theme[role]
                )
                .unwrap();
            }
        }
    }
    o.push_str("</svg>\n");
    o
}
