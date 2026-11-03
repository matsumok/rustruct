//! 等厚曲線の図（コンター図）。横軸に Lx、縦軸に Ly をとり、必要スラブ厚が等しい曲線を引いて間を塗り分け、
//! 検討点を印で示す。言葉は `docs/tools/slab-thickness/GLOSSARY.md`、曲線の引き方はADR slab-thickness-0001。
//!
//! 曲線は `rustruct-calc` の `iso_curves` が x の並びの上で直接引き、`charta` の `level_graphs` が線と帯に
//! する。範囲・厚さの刻み・80 mm の下限・条件の要約は移植元の slab の決めを引き継ぎ、色・線・数値の置き方は
//! サイトの分布図にそろえる（ADR slab-thickness-0002、`docs/ui-decisions.md`「等厚曲線の図」）。図は 1 単位
//! （`width-column`）の幅に収め、色はライトとダークの 2 枚を作る。

use charta::plot::TEXT_SIZE;
use charta::{
    Anchor, Font, HALO_WIDTH, Item, LevelGraph, Paint, Plot, Pt, Scene, Stroke, Theme, axes,
    level_graphs, to_svg,
};
use numerus::at_place;
use rustruct_calc::slab_thickness::{Conditions, Spans, evaluate, iso_curves, shared_values};
use rustruct_types::quantity::Length;

use crate::COLUMN_WIDTH;

/// 検討点が無いときの範囲（両軸、m）
const DEFAULT_VIEW: (f64, f64) = (3.0, 9.0);
/// 範囲の余白（検討点を囲む正方形の半幅に対する比）
const VIEW_MARGIN: f64 = 0.10;
/// 範囲の半幅の下限（m）。検討点が 1 つなら幅 2 m
const MIN_VIEW_HALF: f64 = 1.0;
/// スパンとして扱える下限（m）。範囲が原点をまたいでも、曲線はここから先だけ引く
const MIN_SPAN: f64 = 0.05;
/// 曲線を引く x の数（範囲の幅を等分する）
const SAMPLES: usize = 400;
/// 曲線の厚さの下限（mm）。これ未満の領域は塗らない
const THICKNESS_FLOOR: f64 = 80.0;
/// 曲線の厚さの刻み（mm）。曲線が多すぎるときだけ切りのよい値に粗くする
const THICKNESS_STEP: f64 = 10.0;
/// 一度に引く曲線の本数の上限
const MAX_CURVES: usize = 200;
/// 目盛りの数値の幅の見込み（文字数）
const DIGITS: usize = 4;
/// 条件の要約の行送り
const LINE_HEIGHT: f64 = 16.0;
/// 枠の内側から条件の要約までの空き
const INSET: f64 = 6.0;
/// 検討点の印の半径と線の太さ
const MARK_RADIUS: f64 = 3.5;
const MARK_WIDTH: f64 = 1.0;

/// 等厚曲線を引く素。条件と、検討点の名称とスパン。入力から一度だけ作る
#[derive(Debug, Clone, PartialEq)]
pub struct SlabCurves {
    pub(super) conditions: Conditions,
    /// スパンが読めないときは無し。範囲は検討点の無いもので、印を置かない
    pub(super) point: Option<(String, Spans)>,
}

/// 等厚曲線の図。図の名前はページが置く
#[derive(Debug, Clone, PartialEq)]
pub struct SlabFigure {
    /// 読み上げの文言
    pub label: String,
    pub light_svg: String,
    pub dark_svg: String,
}

impl SlabFigure {
    pub fn new(curves: &SlabCurves) -> Self {
        let point = curves
            .point
            .as_ref()
            .map(|(name, spans)| (name.as_str(), *spans));
        let scene = scene(&curves.conditions, point);
        let svg = |theme| to_svg(&scene, &theme).expect("役割名は tokens.json にある");
        let label = match point {
            Some((name, _)) if !name.is_empty() => {
                format!("必要スラブ厚の等厚曲線。横軸は Lx、縦軸は Ly。丸の印は検討点 {name}")
            }
            Some(_) => "必要スラブ厚の等厚曲線。横軸は Lx、縦軸は Ly。丸の印は検討点".into(),
            None => "必要スラブ厚の等厚曲線。横軸は Lx、縦軸は Ly".into(),
        };
        Self {
            label,
            light_svg: svg(Theme::light()),
            dark_svg: svg(Theme::dark()),
        }
    }
}

/// 範囲（m）。x と y は同じ幅の正方形
#[derive(Debug, Clone, Copy, PartialEq)]
struct View {
    x: (f64, f64),
    y: (f64, f64),
}

impl View {
    /// 検討点の重心を中心に、全点が入る正方形と余白。点が無ければ両軸 3〜9 m、
    /// 点が 1 つ（や同じ点だけ）なら幅 2 m
    fn around(points: &[(f64, f64)]) -> Self {
        if points.is_empty() {
            return Self {
                x: DEFAULT_VIEW,
                y: DEFAULT_VIEW,
            };
        }
        let n = points.len() as f64;
        let cx = points.iter().map(|p| p.0).sum::<f64>() / n;
        let cy = points.iter().map(|p| p.1).sum::<f64>() / n;
        let reach = points
            .iter()
            .map(|(x, y)| (x - cx).abs().max((y - cy).abs()))
            .fold(0.0, f64::max);
        let half = (reach * (1.0 + VIEW_MARGIN)).max(MIN_VIEW_HALF);
        Self {
            x: (cx - half, cx + half),
            y: (cy - half, cy + half),
        }
    }
}

fn scene(conditions: &Conditions, point: Option<(&str, Spans)>) -> Scene {
    let m = Length::as_m;
    let points: Vec<(f64, f64)> = point.iter().map(|(_, s)| (m(s.x), m(s.y))).collect();
    let view = View::around(&points);

    let [left, top, right, bottom] = Plot::margins(DIGITS);
    let side = COLUMN_WIDTH - left - right;
    let plot = Plot {
        x: view.x,
        y: view.y,
        origin: [left, top],
        size: [side, side],
    };
    let mut scene = Scene::new(COLUMN_WIDTH, top + side + bottom);

    // 曲線は範囲のうちスパンとして意味のある所だけ
    let from = view.x.0.max(MIN_SPAN);
    let xs: Vec<f64> = (0..=SAMPLES)
        .map(|i| from + (view.x.1 - from) * i as f64 / SAMPLES as f64)
        .collect();
    let thickness_at = |x: f64, y: f64| {
        let spans = Spans {
            x: Length::m(x),
            y: Length::m(y),
        };
        evaluate(conditions, spans)
            .expect("条件は読めている")
            .required_thickness
            .as_mm()
    };
    // 必要スラブ厚は Lx にも Ly にも増えるので、範囲の厚さは左下と右上の隅で決まる
    let t_min = thickness_at(from, view.y.0.max(MIN_SPAN));
    let t_max = thickness_at(view.x.1, view.y.1);
    let (thicknesses, step) = thicknesses(t_min, t_max);
    let curves = iso_curves(
        conditions,
        &xs.iter().map(|&x| Length::m(x)).collect::<Vec<_>>(),
        &thicknesses
            .iter()
            .map(|&t| Length::mm(t))
            .collect::<Vec<_>>(),
    )
    .expect("条件は読めている");
    let graphs: Vec<LevelGraph> = curves
        .into_iter()
        .map(|curve| LevelGraph {
            level: curve.thickness.as_mm(),
            ys: curve.ys.into_iter().map(|y| y.map(m)).collect(),
        })
        .collect();
    // 図の上に重ねる文字と印。等厚曲線の数値はここを避ける
    let mut overlay: Vec<Item> = Vec::new();
    if let Some((name, spans)) = point {
        let at = plot.to_screen([m(spans.x), m(spans.y)]);
        overlay.push(Item::Circle {
            center: at,
            radius: MARK_RADIUS,
            fill: Some(Paint::role("paper")),
            stroke: Some(Stroke::new("ink", MARK_WIDTH)),
        });
        if !name.is_empty() {
            overlay.push(text(
                [at[0] + 2.0 * MARK_RADIUS, at[1] - 2.0 * MARK_RADIUS],
                name,
            ));
        }
    }
    for (k, line) in summary(conditions).into_iter().enumerate() {
        overlay.push(text(
            [
                left + INSET,
                top + INSET + TEXT_SIZE / 2.0 + k as f64 * LINE_HEIGHT,
            ],
            &line,
        ));
    }
    let clear: Vec<[Pt; 2]> = overlay.iter().filter_map(extent).collect();
    // 色は曲線 2 本ごとに変わる
    scene.place(
        level_graphs(&plot, &xs, &graphs, 2.0 * step, &clear),
        [0.0, 0.0],
    );
    scene.items.extend(overlay);
    scene.place(axes(&plot, "Lx（m）", "Ly（m）", DIGITS), [0.0, 0.0]);
    scene
}

/// 印と文字が占める矩形（左上と右下）に、白抜きの縁の分を足したもの。文字の幅は charta の見込み
fn extent(item: &Item) -> Option<[Pt; 2]> {
    let pad = HALO_WIDTH;
    match item {
        Item::Circle { center, radius, .. } => {
            let r = radius + MARK_WIDTH + pad;
            Some([
                [center[0] - r, center[1] - r],
                [center[0] + r, center[1] + r],
            ])
        }
        Item::Text {
            x,
            y,
            text,
            size,
            font,
            ..
        } => {
            let width = font.width(text, *size);
            Some([
                [x - pad, y - size / 2.0 - pad],
                [x + width + pad, y + size / 2.0 + pad],
            ])
        }
        _ => None,
    }
}

/// 図の中の文字（条件の要約、検討点の名前）。目盛りの数値と同じ `label` の段の大きさの、`ink` の和文を `paper` の縁で白抜きにし、左に寄せる
fn text(at: [f64; 2], s: &str) -> Item {
    Item::Text {
        x: at[0],
        y: at[1],
        text: s.to_string(),
        size: TEXT_SIZE,
        font: Font::Sans,
        fill: Paint::role("ink"),
        anchor: Anchor::Start,
        rotate: 0.0,
        halo: Some(Stroke::new("paper", HALO_WIDTH)),
    }
}

/// 範囲の厚さ [t_min, t_max]（mm）を覆う曲線の厚さと、その刻み。刻みは本数が上限に収まる切りのよい値
/// （10 mm 以上）。80 mm 未満の曲線は作らず、80 mm を最も薄い曲線にする
fn thicknesses(t_min: f64, t_max: f64) -> (Vec<f64>, f64) {
    let lo = t_min.max(THICKNESS_FLOOR);
    let hi = t_max.max(lo);
    let mut step = nice_step(((hi - lo) / (MAX_CURVES - 2) as f64).max(THICKNESS_STEP));
    let mut ts = between(lo, hi, step);
    // 丸めの端数で 1 本はみ出しうるので、収まるまで倍々に粗くする
    while ts.len() > MAX_CURVES {
        step *= 2.0;
        ts = between(lo, hi, step);
    }
    (ts, step)
}

/// `min_step` 以上で最小の切りのよい刻み（…, 10, 25, 50, 100, 250, …）
fn nice_step(min_step: f64) -> f64 {
    let magnitude = 10f64.powf(min_step.log10().floor());
    [1.0, 2.5, 5.0]
        .into_iter()
        .map(|m| m * magnitude)
        .find(|&s| s >= min_step)
        .unwrap_or(10.0 * magnitude)
}

/// [lo, hi] を覆う厚さ。lo を刻みに切り下げた値（80 mm 未満なら 80 mm）から、hi 以上の刻みの点まで。
/// 帯を作れるよう 2 本以上
fn between(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let lo = ((lo / step).floor() * step).max(THICKNESS_FLOOR);
    let first = (lo / step).floor() as i64 + 1;
    let last = ((hi / step).ceil() as i64).max(first);
    std::iter::once(lo)
        .chain((first..=last).map(|k| k as f64 * step))
        .collect()
}

/// 条件の要約の 2 行。Fc と wp、変形増大係数 α と有効なクライテリアだけ。単位は入力の欄と同じで、書かない
fn summary(conditions: &Conditions) -> [String; 2] {
    let wp = shared_values(
        conditions.concrete,
        conditions.finish_load,
        conditions.live_load,
    )
    .expect("条件は読めている")
    .wp;
    let mut criteria = vec![format!("α={}", plain(conditions.amplification))];
    if let Some(ratio) = conditions.criteria.deflection_ratio {
        criteria.push(format!("δ/lx≤1/{}", plain(1.0 / ratio)));
    }
    if let Some(limit) = conditions.criteria.deflection_absolute {
        criteria.push(format!("δ≤{}", plain(limit.as_mm())));
    }
    if conditions.criteria.min_thickness {
        criteria.push("表18-1".into());
    }
    [
        format!(
            "Fc={}{GAP}wp={}",
            plain(conditions.concrete.fc().as_n_per_mm2()),
            plain(wp.as_n_per_m2())
        ),
        criteria.join(GAP),
    ]
}

/// 要約の項目の間。SVG は続く空白を 1 つに詰めるので、和文の空白で空ける
const GAP: &str = "\u{3000}";

/// 打った値に近い書き方。小数 6 桁で丸め（1/n を戻した端数を落とす）、末尾の 0 と小数点を消す
fn plain(x: f64) -> String {
    let s = at_place(x, -6);
    if s.contains('.') && !s.contains('E') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}
