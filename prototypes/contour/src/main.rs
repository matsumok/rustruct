//! 試作（Issue #37）：分布図の見た目の候補を SVG と PNG に書く。捨てるコード。
//!
//! 継ぎ目は charta と同じ形にする：等間隔グリッド → 等値線と帯（contour）→ シーン → SVG（scene）。
//! 使い方：`cargo run --release -- ../../docs/design/tokens.json out fonts`
//! fonts には IBM Plex Mono と IBM Plex Sans JP の ttf を置く（Google Fonts の配布物。リポジトリには入れない）。
//! 決定は Issue #37 のコメントと docs/ui-decisions.md。出力の Z-decided-* が決定どおりの図。

mod contour;
mod scene;

use std::f64::consts::PI;
use std::path::Path;

use contour::{Grid, Pt, contour};
use scene::{Font, Item, Scene, Stroke, Theme, to_svg};

// ---- 合成データ（物の値に近い大きさにする） ----

const Q: f64 = 10.0; // kN/m²
const E: f64 = 24.0e6; // kN/m²
const T: f64 = 0.2; // m
const NU: f64 = 0.2;

fn d() -> f64 {
    E * T.powi(3) / (12.0 * (1.0 - NU * NU))
}

/// 四辺単純支持のたわみ（Navier の級数、mm）。一方向の量の見本。
fn w_simple(lx: f64, ly: f64) -> impl Fn(f64, f64) -> f64 {
    move |x, y| {
        let mut w = 0.0;
        for m in (1..40).step_by(2) {
            for n in (1..40).step_by(2) {
                let (m, n) = (m as f64, n as f64);
                let k = (m / lx).powi(2) + (n / ly).powi(2);
                w += (m * PI * x / lx).sin() * (n * PI * y / ly).sin() / (m * n * k * k);
            }
        }
        16.0 * Q / (PI.powi(6) * d()) * w * 1000.0
    }
}

/// 四辺固定の Mx（w = c·x²(Lx−x)²·y²(Ly−y)² の 1 項 Galerkin、kN·m/m、下側引張が正）。正負のある量の見本。
fn mx_fixed(lx: f64, ly: f64) -> impl Fn(f64, f64) -> f64 {
    let p = |x: f64, l: f64| x * x * (l - x).powi(2);
    let p2 = |x: f64, l: f64| 2.0 * (l * l - 6.0 * l * x + 6.0 * x * x);
    // c = q∫XY / D∫(X''''Y + 2X''Y'' + XY'''')XY を数値積分で。
    let n = 400;
    let (mut num, mut den) = (0.0, 0.0);
    for i in 0..n {
        for j in 0..n {
            let x = (i as f64 + 0.5) * lx / n as f64;
            let y = (j as f64 + 0.5) * ly / n as f64;
            let (xx, yy) = (p(x, lx), p(y, ly));
            num += xx * yy;
            den += (24.0 * yy + 2.0 * p2(x, lx) * p2(y, ly) + 24.0 * xx) * xx * yy;
        }
    }
    let c = Q * num / (d() * den);
    move |x, y| -d() * c * (p2(x, lx) * p(y, ly) + NU * p(x, lx) * p2(y, ly))
}

// ---- 色 ----

fn hex(s: &str) -> [f64; 3] {
    let v = |k: usize| u8::from_str_radix(&s[k..k + 2], 16).unwrap() as f64 / 255.0;
    [v(1), v(3), v(5)]
}

fn to_hex(c: [f64; 3]) -> String {
    let b = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
}

fn oklab(c: [f64; 3]) -> [f64; 3] {
    let lin = |v: f64| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    let [r, g, b] = c.map(lin);
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

fn from_oklab(c: [f64; 3]) -> [f64; 3] {
    let l = (c[0] + 0.3963377774 * c[1] + 0.2158037573 * c[2]).powi(3);
    let m = (c[0] - 0.1055613458 * c[1] - 0.0638541728 * c[2]).powi(3);
    let s = (c[0] - 0.0894841775 * c[1] - 1.2914855480 * c[2]).powi(3);
    let srgb = |v: f64| if v <= 0.0031308 { 12.92 * v } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    [
        srgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
        srgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
        srgb(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s),
    ]
}

/// 折れ線状の色の道を OKLab で n 等分した点の色。
fn ramp(anchors: &[[f64; 3]], n: usize) -> Vec<String> {
    let labs: Vec<_> = anchors.iter().map(|&c| oklab(c)).collect();
    (0..n)
        .map(|k| {
            let t = if n == 1 { 0.0 } else { k as f64 / (n - 1) as f64 } * (labs.len() - 1) as f64;
            let i = (t.floor() as usize).min(labs.len() - 2);
            let f = t - i as f64;
            let c = [0, 1, 2].map(|q| labs[i][q] + f * (labs[i + 1][q] - labs[i][q]));
            to_hex(from_oklab(c))
        })
        .collect()
}

fn luminance(c: [f64; 3]) -> f64 {
    let lin = |v: f64| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

fn contrast(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn mix(a: &str, b: &str) -> String {
    let (a, b) = (oklab(hex(a)), oklab(hex(b)));
    to_hex(from_oklab([0, 1, 2].map(|q| (a[q] + b[q]) / 2.0)))
}

#[derive(Clone, Copy, PartialEq)]
enum Palette {
    /// 正負で色相を分ける（div-1〜12、藍←灰→琥珀）
    Div,
    /// 単色の濃淡（seq-1〜12、藍）
    Seq,
    /// 虹（藍→青紫→青碧→琥珀→淡い黄）
    Rainbow,
}

/// 帯の色を下から順に。div と seq は 12 段の間を補間して取る（ui-decisions の「段数を変えるときは補間」）。
fn band_colors(theme: &Theme, dark: bool, pal: Palette, edges: &[f64]) -> Vec<String> {
    let nb = edges.len() - 1;
    let tok = |name: &str| hex(&theme[name]);
    match pal {
        Palette::Seq => ramp(&(1..=12).map(|k| tok(&format!("seq-{k}"))).collect::<Vec<_>>(), nb),
        Palette::Div => {
            // 0 を境に左右を別々に取る。範囲は 0 を中心に対称（edges もそう作る）。
            let h = nb / 2;
            let neg = ramp(&(1..=6).map(|k| tok(&format!("div-{k}"))).collect::<Vec<_>>(), h);
            let pos = ramp(&(7..=12).map(|k| tok(&format!("div-{k}"))).collect::<Vec<_>>(), h);
            neg.into_iter().chain(pos).collect()
        }
        Palette::Rainbow => {
            // 藍は seq-12 のライトの値（藍の深い段）、青紫 chart-5、青碧 chart-2、琥珀 chart-1 のダークの値、淡い黄 amber-3。
            let mut a = vec![hex("#1f2d5c"), tok("chart-5"), tok("chart-2"), hex("#ffca16"), hex("#fff7c2")];
            // 値が大きいほど地から遠く（seq と同じ向き）：ライトは淡い黄から、ダークは藍から。
            if !dark {
                a.reverse();
            }
            ramp(&a, nb)
        }
    }
}

fn load_theme(tokens: &serde_json::Value, mode: &str) -> Theme {
    let mut t = Theme::new();
    for tok in tokens["color"]["tokens"].as_array().unwrap() {
        t.insert(tok["name"].as_str().unwrap().into(), tok["value"][mode].as_str().unwrap().into());
    }
    let refs: Vec<_> = t.iter().filter(|(_, v)| v.starts_with('{')).map(|(k, v)| (k.clone(), v.clone())).collect();
    for (k, v) in refs {
        let target = t[v.trim_matches(|c| c == '{' || c == '}')].clone();
        t.insert(k, target);
    }
    t
}

// ---- 目盛りと数値 ----

fn nice_step(span: f64, target: usize) -> f64 {
    let raw = span / target as f64;
    let p = 10f64.powf(raw.log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|m| m * p).find(|&s| s >= raw * 0.999).unwrap()
}

fn edges_for(pal: Palette, lo: f64, hi: f64, target: usize) -> (Vec<f64>, f64) {
    if pal == Palette::Div {
        let m = lo.abs().max(hi.abs());
        let step = nice_step(2.0 * m, target);
        let k = (m / step - 1e-9).ceil() as i64;
        ((-k..=k).map(|i| i as f64 * step).collect(), step)
    } else {
        let step = nice_step(hi - lo, target);
        let a = (lo / step + 1e-9).floor() as i64;
        let b = (hi / step - 1e-9).ceil() as i64;
        ((a..=b).map(|i| i as f64 * step).collect(), step)
    }
}

fn fmt(v: f64, step: f64) -> String {
    let dec = (-(step.log10().floor()) as i64).max(0) as usize + if (step / 10f64.powf(step.log10().floor()) - 2.5).abs() < 1e-9 { 1 } else { 0 };
    let s = format!("{:.*}", dec, v.abs());
    if v < 0.0 && s.chars().any(|c| c != '0' && c != '.') { format!("−{s}") } else { s }
}

// ---- 図 ----

#[derive(Clone)]
struct Style {
    palette: Palette,
    bands: usize,
    line_w: f64,
    line_role: &'static str,
    /// 0 の線（div のとき）の太さ
    zero_w: f64,
    label_size: f64,
    halo_w: f64,
    /// 数値を線に沿って傾けず、水平に置く
    upright: bool,
    density: Density,
    /// 刻みが 1 未満のとき、整数の線にだけ数値を付ける
    int_only: bool,
    /// 縁を地の色でなく、その場の帯の色（線の両側の中間）にする。文字色は ink と paper のうち帯との差が大きい方
    halo_band: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Density {
    /// 線の中ほどに置き、近い数値とぶつかれば捨てる
    Sparse,
    /// すべての線に付ける。ぶつかれば線の上で場所をずらす
    All,
    /// 1 本おきの線に付ける（ずらすのは All と同じ）
    Alternate,
    /// 板の中心から辺か角へ引いた案内線（画面には描かない）との交点に並べる。終点を板の座標の割合で持つ
    Guide(&'static [(f64, f64)]),
}

struct Field {
    title: &'static str,
    lx: f64,
    ly: f64,
    grid: Grid,
}

fn field(signed: bool, lx: f64, ly: f64) -> Field {
    let n = |l: f64| (l / 0.1).round() as usize + 1;
    let axis = |l: f64| (0..n(l)).map(|i| l * i as f64 / (n(l) - 1) as f64).collect::<Vec<_>>();
    let grid = if signed {
        Grid::from_fn(axis(lx), axis(ly), mx_fixed(lx, ly))
    } else {
        Grid::from_fn(axis(lx), axis(ly), w_simple(lx, ly))
    };
    let title = if signed { "曲げモーメント Mx（kN·m/m）" } else { "たわみ w（mm）" };
    Field { title, lx, ly, grid }
}

/// 一つの図。theme は帯の色（band-k）を書き足すので可変で受ける。
fn figure(f: &Field, st: &Style, theme: &mut Theme, dark: bool) -> Scene {
    let (pw, ph) = if f.lx == f.ly { (300.0, 300.0) } else { (480.0, 480.0 * f.ly / f.lx) };
    let (left, top, cb_gap, cb_w, right, bottom) = (44.0, 36.0, 18.0, 12.0, 52.0, 40.0);
    let w = left + pw + cb_gap + cb_w + right;
    let h = top + ph + bottom;
    let tx = |p: Pt| -> Pt { [left + p[0] / f.lx * pw, top + ph - p[1] / f.ly * ph] };

    let (lo, hi) = f.grid.range();
    let (edges, step) = edges_for(st.palette, lo, hi, st.bands);
    let colors = band_colors(theme, dark, st.palette, &edges);
    for (k, c) in colors.iter().enumerate() {
        theme.insert(format!("band-{k}"), c.clone());
    }
    let (isolines, bands) = contour(&f.grid, &edges);
    let s = |r: &str, w: f64| Some(Stroke { role: r.into(), width: w });

    let mut items = vec![];
    items.push(Item::Text {
        x: left,
        y: 16.0,
        s: f.title.into(),
        size: 13.0,
        font: Font::Sans,
        role: "ink".into(),
        anchor: "start",
        rotate: 0.0,
        halo: None,
    });
    for (k, b) in bands.iter().enumerate() {
        if b.polygons.is_empty() {
            continue;
        }
        let subpaths = b.polygons.iter().map(|p| p.iter().map(|&q| tx(q)).collect()).collect();
        items.push(Item::Path { subpaths, closed: true, fill: Some(format!("band-{k}")), stroke: None });
    }
    // 線
    let mut screen_lines: Vec<(f64, Vec<Pt>)> = vec![];
    for iso in &isolines {
        let lines: Vec<Vec<Pt>> = iso.lines.iter().map(|l| l.iter().map(|&q| tx(q)).collect()).collect();
        let lw = if st.palette == Palette::Div && iso.level.abs() < 1e-12 { st.zero_w } else { st.line_w };
        let role = if lw == st.zero_w && st.palette == Palette::Div && iso.level.abs() < 1e-12 { "ink" } else { st.line_role };
        items.push(Item::Path { subpaths: lines.clone(), closed: false, fill: None, stroke: s(role, lw) });
        screen_lines.extend(lines.into_iter().map(|l| (iso.level, l)));
    }
    // 等値線上の数値（白抜き）
    let mut placed: Vec<Pt> = vec![];
    let cw = st.label_size * 0.6;
    if let Density::Guide(ends) = st.density {
      let a = tx([f.lx / 2.0, f.ly / 2.0]);
      for &(gx, gy) in ends {
        let b = tx([f.lx * gx, f.ly * gy]);
        let mut hits: Vec<(f64, Pt, f64, f64)> = vec![]; // 中心からの距離、点、角度、値
        for (level, l) in &screen_lines {
            for w in l.windows(2) {
                let (p, q) = (w[0], w[1]);
                let r = [q[0] - p[0], q[1] - p[1]];
                let g = [b[0] - a[0], b[1] - a[1]];
                let den = r[0] * g[1] - r[1] * g[0];
                if den.abs() < 1e-12 {
                    continue;
                }
                let t = ((a[0] - p[0]) * g[1] - (a[1] - p[1]) * g[0]) / den;
                let u = ((a[0] - p[0]) * r[1] - (a[1] - p[1]) * r[0]) / den;
                if (0.0..1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    let pt = [p[0] + t * r[0], p[1] + t * r[1]];
                    hits.push((u, pt, r[1].atan2(r[0]).to_degrees(), *level));
                }
            }
        }
        hits.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        for (_, p, mut ang, level) in hits {
            if ang > 90.0 {
                ang -= 180.0;
            } else if ang < -90.0 {
                ang += 180.0;
            }
            if st.int_only && step < 1.0 && (level - level.round()).abs() > 1e-9 {
                continue;
            }
            let text = if st.int_only && step < 1.0 { fmt(level, 1.0) } else { fmt(level, step) };
            let tw = text.chars().count() as f64 * cw;
            let free = placed.iter().all(|q| (q[0] - p[0]).hypot(q[1] - p[1]) > tw.max(st.label_size) + st.halo_w * 2.0);
            let margin = 6.0;
            let inside = p[0] > left + margin && p[0] < left + pw - margin && p[1] > top + margin && p[1] < top + ph - margin;
            if free && inside {
                placed.push(p);
                let (mut ink, mut halo) = ("ink".to_string(), "paper".to_string());
                if st.halo_band {
                    let k = edges.iter().position(|e| (e - level).abs() < 1e-9).unwrap();
                    let bg = mix(&colors[k - 1], &colors[k]);
                    let pick = if contrast(hex(&theme["ink"]), hex(&bg)) >= contrast(hex(&theme["paper"]), hex(&bg)) { "ink" } else { "paper" };
                    halo = format!("halo-{k}");
                    ink = pick.to_string();
                    theme.insert(halo.clone(), bg);
                }
                items.push(Item::Text {
                    x: p[0],
                    y: p[1],
                    s: text,
                    size: st.label_size,
                    font: Font::Mono,
                    role: ink,
                    anchor: "middle",
                    rotate: if st.upright { 0.0 } else { ang },
                    halo: s(&halo, st.halo_w),
                });
            }
        }
      }
    }
    for (level, l) in screen_lines.iter().filter(|_| !matches!(st.density, Density::Guide(..))) {
        if st.density == Density::Alternate && ((level / step).round() as i64).rem_euclid(2) != 0 {
            continue;
        }
        let cum: Vec<f64> = std::iter::once(0.0)
            .chain(l.windows(2).scan(0.0, |a, w| {
                *a += ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
                Some(*a)
            }))
            .collect();
        let len = *cum.last().unwrap();
        let text = fmt(*level, step);
        let tw = text.chars().count() as f64 * cw;
        if len < tw * 3.0 {
            continue;
        }
        let n = ((len / 260.0).floor() as usize).max(1);
        let at = |s: f64| -> Pt {
            let i = cum.partition_point(|&c| c < s).clamp(1, l.len() - 1);
            let t = (s - cum[i - 1]) / (cum[i] - cum[i - 1]).max(1e-9);
            [l[i - 1][0] + t * (l[i][0] - l[i - 1][0]), l[i - 1][1] + t * (l[i][1] - l[i - 1][1])]
        };
        // 候補の位置：Sparse は区間の中ほどだけ、ほかは区間の中で中ほどから外へ順に試す。
        let tries: Vec<f64> = if st.density == Density::Sparse { vec![0.5] } else { vec![0.5, 0.35, 0.65, 0.2, 0.8, 0.1, 0.9] };
        let gap = if st.density == Density::Sparse { 16.0 } else { 6.0 };
        for k in 0..n {
          for &fr in &tries {
            let s0 = len * (k as f64 + fr) / n as f64;
            let p = at(s0);
            let (a, b) = (at((s0 - tw / 2.0).max(0.0)), at((s0 + tw / 2.0).min(len)));
            let mut ang = (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees();
            if ang > 90.0 {
                ang -= 180.0;
            } else if ang < -90.0 {
                ang += 180.0;
            }
            let margin = tw / 2.0 + 4.0;
            let inside = p[0] > left + margin && p[0] < left + pw - margin && p[1] > top + margin && p[1] < top + ph - margin;
            let free = placed.iter().all(|q| (q[0] - p[0]).hypot(q[1] - p[1]) > tw + gap);
            if inside && free {
                placed.push(p);
                items.push(Item::Text {
                    x: p[0],
                    y: p[1],
                    s: text.clone(),
                    size: st.label_size,
                    font: Font::Mono,
                    role: "ink".into(),
                    anchor: "middle",
                    rotate: if st.upright { 0.0 } else { ang },
                    halo: s("paper", st.halo_w),
                });
                break;
            }
          }
        }
    }
    // 枠と目盛り（matplotlib 型：四辺の枠、内向きの目盛り、数値は下と左）
    items.push(Item::Rect { x: left, y: top, w: pw, h: ph, fill: None, stroke: s("ink", 1.0) });
    let tstep = if f.lx.max(f.ly) > 8.0 { 2.0 } else { 1.0 };
    let mut tick = |pts: Vec<Pt>| items.push(Item::Path { subpaths: vec![pts], closed: false, fill: None, stroke: s("ink", 1.0) });
    let mut labels = vec![];
    let mut x = 0.0;
    while x <= f.lx + 1e-9 {
        let [sx, _] = tx([x, 0.0]);
        tick(vec![[sx, top + ph], [sx, top + ph - 4.0]]);
        tick(vec![[sx, top], [sx, top + 4.0]]);
        labels.push((sx, top + ph + 12.0, format!("{x}"), "middle"));
        x += tstep;
    }
    let mut y = 0.0;
    while y <= f.ly + 1e-9 {
        let [_, sy] = tx([0.0, y]);
        tick(vec![[left, sy], [left + 4.0, sy]]);
        tick(vec![[left + pw, sy], [left + pw - 4.0, sy]]);
        labels.push((left - 6.0, sy, format!("{y}"), "end"));
        y += tstep;
    }
    labels.push((left + pw / 2.0, top + ph + 30.0, "x（m）".into(), "middle"));
    for (x, y, t, anchor) in labels {
        let font = if t.contains('（') { Font::Sans } else { Font::Mono };
        items.push(Item::Text { x, y, s: t, size: 11.0, font, role: "muted".into(), anchor, rotate: 0.0, halo: None });
    }
    items.push(Item::Text {
        x: left - 30.0,
        y: top + ph / 2.0,
        s: "y（m）".into(),
        size: 11.0,
        font: Font::Sans,
        role: "muted".into(),
        anchor: "middle",
        rotate: -90.0,
        halo: None,
    });
    // カラーバー：帯を同じ高さで積み、境に値。範囲の外（データの無い帯）も描く。
    let cbx = left + pw + cb_gap;
    let nb = edges.len() - 1;
    let bh = ph / nb as f64;
    for k in 0..nb {
        let y = top + ph - (k + 1) as f64 * bh;
        items.push(Item::Rect { x: cbx, y, w: cb_w, h: bh, fill: Some(format!("band-{k}")), stroke: None });
    }
    for (k, e) in edges.iter().enumerate() {
        let y = top + ph - k as f64 * bh;
        if k > 0 && k < nb {
            let lw = if st.palette == Palette::Div && e.abs() < 1e-12 { st.zero_w } else { st.line_w };
            items.push(Item::Path { subpaths: vec![vec![[cbx, y], [cbx + cb_w, y]]], closed: false, fill: None, stroke: s("ink", lw) });
        }
        items.push(Item::Text { x: cbx + cb_w + 6.0, y, s: fmt(*e, step), size: 11.0, font: Font::Mono, role: "muted".into(), anchor: "start", rotate: 0.0, halo: None });
    }
    items.push(Item::Rect { x: cbx, y: top, w: cb_w, h: ph, fill: None, stroke: s("ink", 1.0) });
    Scene { w, h, items }
}

// ---- 一覧の紙（複数の図を並べて 1 枚の PNG に） ----

struct Cell {
    svg: String,
    w: f64,
    h: f64,
    caption: String,
    paper: String,
    muted: String,
}

fn sheet(rows: Vec<Vec<Cell>>, out: &Path, name: &str, fontdb: &std::sync::Arc<resvg::usvg::fontdb::Database>) {
    let cap = 22.0;
    let row_h: Vec<f64> = rows.iter().map(|r| r.iter().map(|c| c.h).fold(0.0, f64::max) + cap).collect();
    let w = rows.iter().map(|r| r.iter().map(|c| c.w).sum::<f64>()).fold(0.0, f64::max);
    let h: f64 = row_h.iter().sum();
    let mut o = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="100%" height="100%" fill="#888"/>"##);
    let mut y = 0.0;
    for (r, row) in rows.iter().enumerate() {
        let mut x = 0.0;
        if let Some(c) = row.first() {
            o += &format!(r#"<rect x="0" y="{y}" width="{w}" height="{}" fill="{}"/>"#, row_h[r], c.paper);
        }
        for c in row {
            o += &format!(r#"<text x="{}" y="{}" font-family="IBM Plex Sans JP" font-size="12" fill="{}">{}</text>"#, x + 8.0, y + 15.0, c.muted, c.caption);
            let inner = c.svg.replacen("<svg ", &format!(r#"<svg x="{x}" y="{}" "#, y + cap), 1);
            o += &inner;
            x += c.w;
        }
        y += row_h[r];
    }
    o += "</svg>";
    std::fs::write(out.join(format!("{name}.svg")), &o).unwrap();
    let opt = resvg::usvg::Options { fontdb: fontdb.clone(), ..Default::default() };
    let tree = resvg::usvg::Tree::from_str(&o, &opt).unwrap();
    let scale = 2.0;
    let mut pix = resvg::tiny_skia::Pixmap::new((w * scale) as u32, (h * scale) as u32).unwrap();
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale as f32, scale as f32), &mut pix.as_mut());
    pix.save_png(out.join(format!("{name}.png"))).unwrap();
    println!("{name}.png");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let tokens: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let out = Path::new(&args[2]);
    std::fs::create_dir_all(out).unwrap();
    let mut db = resvg::usvg::fontdb::Database::new();
    db.load_fonts_dir(args.get(3).map(String::as_str).unwrap_or("fonts"));
    let fontdb = std::sync::Arc::new(db);

    let base = Style { palette: Palette::Seq, bands: 10, line_w: 0.25, line_role: "ink", zero_w: 0.25, label_size: 11.0, halo_w: 3.0, upright: false, density: Density::Sparse, int_only: false, halo_band: false };
    let fields = [field(false, 6.0, 6.0), field(false, 12.0, 6.0), field(true, 6.0, 6.0), field(true, 12.0, 6.0)];

    let cell = |f: &Field, st: &Style, mode: &str, caption: String| {
        let mut theme = load_theme(&tokens, mode);
        let sc = figure(f, st, &mut theme, mode == "dark");
        Cell { svg: to_svg(&sc, &theme), w: sc.w, h: sc.h, caption, paper: theme["paper"].clone(), muted: theme["muted"].clone() }
    };
    let pal_sheet = |name: &str, label: &str, st: &Style, signed: bool| {
        let fs = if signed { [&fields[2], &fields[3]] } else { [&fields[0], &fields[1]] };
        let rows = ["light", "dark"]
            .iter()
            .map(|&mode| fs.iter().map(|f| cell(f, st, mode, format!("{label} — {mode} — {}:{}", 1, f.lx / f.ly))).collect())
            .collect();
        sheet(rows, out, name, &fontdb);
    };
    pal_sheet("A-div-Mx", "A 正負で色相を分ける（div）", &Style { palette: Palette::Div, ..base.clone() }, true);
    pal_sheet("B-seq-w", "B 単色の濃淡（seq）", &base, false);
    // 段数の比べ：目安の本数を変えて、きりのよい刻みに丸めた結果の帯の数を見出しに書く。
    let rows = [(&fields[0], Palette::Seq), (&fields[2], Palette::Div)]
        .iter()
        .map(|&(f, palette)| {
            [5, 10, 16]
                .iter()
                .map(|&bands| {
                    let st = Style { palette, bands, ..base.clone() };
                    let (lo, hi) = f.grid.range();
                    let (edges, step) = edges_for(palette, lo, hi, bands);
                    cell(f, &st, "light", format!("帯 {} 段（刻み {step}）", edges.len() - 1))
                })
                .collect()
        })
        .collect();
    sheet(rows, out, "D-bands", &fontdb);
    let variants: [(&str, f64, &'static str); 4] =
        [("ink 0.5px（今）", 0.5, "ink"), ("ink 0.25px", 0.25, "ink"), ("ink 1px", 1.0, "ink"), ("line-strong 0.5px", 0.5, "line-strong")];
    let rows = ["light", "dark"]
        .iter()
        .map(|&mode| {
            variants
                .iter()
                .map(|&(label, line_w, line_role)| {
                    let st = Style { palette: Palette::Div, bands: 16, line_w, line_role, ..base.clone() };
                    cell(&fields[2], &st, mode, format!("{label} — {mode}"))
                })
                .collect()
        })
        .collect();
    sheet(rows, out, "E-lines", &fontdb);
    let variants: [(&str, f64, f64, bool); 4] =
        [("11px・縁 3・傾ける（今）", 11.0, 3.0, false), ("11px・縁 2・傾ける", 11.0, 2.0, false), ("11px・縁 3・水平", 11.0, 3.0, true), ("13px・縁 3・傾ける", 13.0, 3.0, false)];
    let rows = ["light", "dark"]
        .iter()
        .map(|&mode| {
            variants
                .iter()
                .map(|&(label, label_size, halo_w, upright)| {
                    let st = Style { palette: Palette::Div, bands: 16, label_size, halo_w, upright, ..base.clone() };
                    cell(&fields[2], &st, mode, format!("{label} — {mode}"))
                })
                .collect()
        })
        .collect();
    sheet(rows, out, "F-labels", &fontdb);
    let variants = [
        ("ぶつかれば捨てる（今）", Density::Sparse),
        ("案内線：中心→右の辺", Density::Guide(&[(1.0, 0.5)])),
        ("案内線：中心→上の辺", Density::Guide(&[(0.5, 1.0)])),
        ("案内線：中心→右上の角", Density::Guide(&[(1.0, 1.0)])),
    ];
    let rows = [(&fields[2], Palette::Div, 16), (&fields[3], Palette::Div, 16), (&fields[0], Palette::Seq, 10), (&fields[1], Palette::Seq, 10)]
        .iter()
        .map(|&(f, palette, bands)| {
            variants
                .iter()
                .map(|&(label, density)| {
                    let st = Style { palette, bands, density, ..base.clone() };
                    cell(f, &st, "light", label.to_string())
                })
                .collect()
        })
        .collect();
    sheet(rows, out, "G-density", &fontdb);
    let variants = [("案内線：中心→右と上", Density::Guide(&[(1.0, 0.5), (0.5, 1.0)])), ("案内線：中心→四辺", Density::Guide(&[(1.0, 0.5), (0.5, 1.0), (0.0, 0.5), (0.5, 0.0)]))];
    let rows = [(&fields[2], Palette::Div, 16), (&fields[3], Palette::Div, 16), (&fields[0], Palette::Seq, 10), (&fields[1], Palette::Seq, 10)]
        .chunks(2)
        .map(|fs| {
            fs.iter()
                .flat_map(|&(f, palette, bands)| {
                    variants.iter().map(move |&(label, density)| (f, palette, bands, label, density))
                })
                .map(|(f, palette, bands, label, density)| {
                    let st = Style { palette, bands, density, ..base.clone() };
                    cell(f, &st, "light", label.to_string())
                })
                .collect()
        })
        .collect();
    sheet(rows, out, "H-cross", &fontdb);
    const CROSS: &[(f64, f64)] = &[(1.0, 0.5), (0.5, 1.0), (0.0, 0.5), (0.5, 0.0)];
    let g = Style { density: Density::Guide(CROSS), ..base.clone() };
    let variants = [
        ("11px（今）", g.clone()),
        ("9px", Style { label_size: 9.0, halo_w: 2.5, ..g.clone() }),
        ("整数の線にだけ", Style { int_only: true, ..g.clone() }),
        ("水平に置く", Style { upright: true, ..g.clone() }),
    ];
    let rows = [&fields[0], &fields[2]]
        .iter()
        .map(|&f| {
            variants
                .iter()
                .map(|(label, st)| {
                    let st = if std::ptr::eq(f, &fields[2]) { Style { palette: Palette::Div, bands: 16, ..st.clone() } } else { st.clone() };
                    cell(f, &st, "light", label.to_string())
                })
                .collect()
        })
        .collect();
    sheet(rows, out, "I-decimals", &fontdb);
    // 決定：w は seq・目安 10、Mx は div・目安 16、線は ink 0.25px（0 も同じ）、数値は 9px・縁 2.5・線に沿って傾け、中心から四辺への案内線に並べる。
    let decided = Style { label_size: 9.0, halo_w: 2.5, density: Density::Guide(CROSS), ..base.clone() };
    for (name, signed) in [("Z-decided-w", false), ("Z-decided-Mx", true)] {
        let st = if signed { Style { palette: Palette::Div, bands: 16, ..decided.clone() } } else { decided.clone() };
        pal_sheet(name, "決定", &st, signed);
    }
    let variants = [
        ("a 縁は地の色・2.5（今）", decided.clone()),
        ("b 縁は地の色・1.5", Style { halo_w: 1.5, ..decided.clone() }),
        ("c 縁は帯の色・2.5", Style { halo_band: true, ..decided.clone() }),
    ];
    let rows = [("light", &fields[0], false), ("dark", &fields[0], false), ("light", &fields[2], true), ("dark", &fields[2], true)]
        .iter()
        .map(|&(mode, f, signed)| {
            variants
                .iter()
                .map(|(label, st)| {
                    let st = if signed { Style { palette: Palette::Div, bands: 16, ..st.clone() } } else { st.clone() };
                    cell(f, &st, mode, format!("{label} — {mode}"))
                })
                .collect()
        })
        .collect();
    sheet(rows, out, "J-halo", &fontdb);
    pal_sheet("C-rainbow-w", "C 虹", &Style { palette: Palette::Rainbow, ..base.clone() }, false);
}
