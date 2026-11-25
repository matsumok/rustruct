//! 必要スラブ厚さ（Issue #17）: `/tools` の一覧と `/tools/slab-thickness` のページ。
//! 言葉は `docs/tools/slab-thickness/GLOSSARY.md`。数値は `rustruct-calc` が出し、ここでは表示の文字列を見る。

mod common;
use charta::{Font, Item, Pt, Scene};
use common::markup::count_tags;
use common::{
    Said, assert_in_order, element, flagged_heads, input_groups, paragraphs, said, text_of,
};
use numerus::fixed;
use rustruct_calc::slab_thickness::{Conditions, CriteriaSet, Spans, check};
use rustruct_types::concrete::{Concrete, ConcreteKind};
use rustruct_types::quantity::{AreaLoad, Length, Stress};
use rustruct_web::{
    PointTerm, Pointed, Pointer, SlabField, SlabFigure, SlabInputs, SlabOutcome, SlabReadout,
    render_path, render_path_with,
};

/// 図の下の点の値の行の、何も指していないときの読み方の文
const HINT: &str = "図を指すとその点の値を出す";

#[test]
fn tools_lists_slab_thickness_under_the_rc_heading_before_mechanics() {
    let html = render_path("/tools");
    assert_in_order(
        &text_of(element(&html, "main")),
        &[
            "RC",
            "/tools/slab-thickness",
            "必要スラブ厚さ",
            "Mechanics",
            "/tools/kirchhoff-plate",
        ],
    );
    assert!(
        html.contains("href=\"/tools/slab-thickness\""),
        "必要スラブ厚さへのリンクが無い: {html}"
    );
}

#[test]
fn the_tool_page_is_headed_with_its_name() {
    // 住所（/tools/slab-thickness）はヘッダーのパンくずで示す（render.rs）
    let html = render_path("/tools/slab-thickness");
    assert!(html.contains("<title>必要スラブ厚さ · rustruct.matsumok.com</title>"));
    assert!(text_of(element(&html, "main")).starts_with("必要スラブ厚さ"));
}

#[test]
fn the_page_opens_with_the_initial_conditions_and_point() {
    let html = render_path("/tools/slab-thickness");
    let main = element(&html, "main");
    assert_in_order(
        &text_of(main),
        &[
            "Fc（N/mm²）",
            "仕上げ荷重（N/m²）",
            "積載荷重（N/m²）",
            "変形増大係数",
            "変形角 1/",
            "たわみ絶対量（mm）",
            "表18-1 最小厚さ",
            "共有値",
            "名称",
            "Lx（m）",
            "Ly（m）",
        ],
    );
    for value in ["24", "1000", "1800", "16", "400", "", "S1", "4.0", "5.0"] {
        assert!(
            main.contains(&format!("value=\"{value}\"")),
            "入力欄に初期値「{value}」が無い: {main}"
        );
    }
    // 表18-1 はチェックの欄で、初期オフ
    assert_eq!(main.matches("role=\"checkbox\"").count(), 1, "{main}");
    assert!(
        main.contains("aria-checked=\"false\""),
        "表18-1 がオンで開く: {main}"
    );
}

#[test]
fn the_page_shows_the_shared_values_and_the_point_results_in_order() {
    let text = text_of(element(&render_path("/tools/slab-thickness"), "main"));
    assert_in_order(
        &text,
        &[
            "共有値",
            "E（N/mm²）",
            "γ₁（kN/m³）",
            "wp（N/m²）",
            "2.27E+04",
            "24.0",
            "2,800",
            "必要スラブ厚",
            "t_req（mm）",
            "決定条件",
            "変形角",
            "短辺スパンと辺長比",
            "lx（m）",
            "λ",
            "4.00",
            "1.25",
            "たわみ（mm）",
            "δe",
            "増大後",
            "許容",
            "10.00",
        ],
    );
    assert!(!text.contains('—'), "初期値で計算できない値がある: {text}");
    assert!(!text.contains("入力"), "部の見出し「入力」がある: {text}");
    assert!(!text.contains("結果"), "部の見出し「結果」がある: {text}");
}

fn inputs(change: impl FnOnce(&mut SlabInputs)) -> SlabInputs {
    let mut inputs = SlabInputs::default();
    change(&mut inputs);
    inputs
}

fn outcome(change: impl FnOnce(&mut SlabInputs)) -> SlabOutcome {
    SlabOutcome::from_inputs(&inputs(change))
}

fn readout(change: impl FnOnce(&mut SlabInputs)) -> SlabReadout {
    outcome(change).readout
}

/// 入力を打った後のページの、入力の組ごとの HTML（条件の組、検討点の組）
fn groups(change: impl FnOnce(&mut SlabInputs)) -> [String; 2] {
    let html = render_path_with("/tools/slab-thickness", inputs(change));
    let groups = input_groups(element(&html, "main"));
    let [conditions, point] = groups[..] else {
        panic!("入力の組が 2 つでない: {html}");
    };
    [conditions.to_string(), point.to_string()]
}

fn point_values(r: &SlabReadout) -> [Option<&str>; 7] {
    let p = &r.point;
    [
        &p.required_thickness,
        &p.governing,
        &p.short_span,
        &p.aspect_ratio,
        &p.elastic_deflection,
        &p.amplified_deflection,
        &p.allowable_deflection,
    ]
    .map(Option::as_deref)
}

fn shared_values(r: &SlabReadout) -> [Option<&str>; 3] {
    let s = &r.shared;
    [&s.young_modulus, &s.rc_unit_weight, &s.wp].map(Option::as_deref)
}

#[test]
fn the_initial_point_is_governed_by_the_deflection_ratio_at_its_limit() {
    let o = outcome(|_| {});
    assert_eq!(said(&o.record), Said::default());
    let p = &o.readout.point;
    assert_eq!(p.governing.as_deref(), Some("変形角"));
    // 変形角が決めるので、増大後たわみ = 許容たわみ = 4000/400
    assert_eq!(p.amplified_deflection.as_deref(), Some("10.00"));
    assert_eq!(p.allowable_deflection.as_deref(), Some("10.00"));
    // 弾性たわみ = 10 / 変形増大係数 16 = 0.625 ちょうど。四捨五入で 0.63（ADR site-0013）
    assert_eq!(p.elastic_deflection.as_deref(), Some("0.63"));
}

#[test]
fn swapping_lx_and_ly_gives_the_same_results() {
    let swapped = readout(|i| {
        i.set(SlabField::SpanX, "5.0");
        i.set(SlabField::SpanY, "4.0");
    });
    assert_eq!(swapped.point, readout(|_| {}).point);
}

#[test]
fn turning_on_the_table_and_blanking_the_ratio_leaves_only_the_minimum_thickness() {
    let o = outcome(|i| {
        i.set(SlabField::DeflectionRatio, "");
        i.set(SlabField::MinThickness, "true");
    });
    assert_eq!(said(&o.record), Said::default());
    let r = o.readout;
    // wp = 2800 N/m²、lx = 4000 mm、λ = 1.25: 0.02·(0.55/0.65)·(1+0.28+0.4)·4000 = 113.7
    assert_eq!(r.point.required_thickness.as_deref(), Some("113.7"));
    assert_eq!(r.point.governing.as_deref(), Some("表18-1"));
    // たわみに効く条件が無いので、許容たわみは無い
    assert_eq!(r.point.allowable_deflection, None);
}

#[test]
fn blank_criteria_with_the_table_off_is_an_error_and_nothing_is_evaluated() {
    let o = outcome(|i| i.set(SlabField::DeflectionRatio, ""));
    assert_eq!(
        said(&o.record),
        Said {
            across: vec![(
                vec![
                    SlabField::DeflectionRatio,
                    SlabField::DeflectionAbsolute,
                    SlabField::MinThickness,
                ],
                "クライテリアを 1 つ以上入れる",
            )],
            ..Said::default()
        }
    );
    let r = o.readout;
    assert!(point_values(&r).iter().all(Option::is_none), "{r:#?}");
    // 共有値は条件だけで決まるので出る
    assert_eq!(
        shared_values(&r),
        [Some("2.27E+04"), Some("24.0"), Some("2,800")]
    );
}

#[test]
fn an_unreadable_span_leaves_the_shared_values_and_dashes_the_point() {
    let o = outcome(|i| {
        i.set(SlabField::SpanX, "0");
        i.set(SlabField::SpanY, "abc");
    });
    assert_eq!(
        said(&o.record),
        Said {
            reasons: vec![
                (SlabField::SpanX, "0 より大きい値を入れる"),
                (SlabField::SpanY, "数値で入れる"),
            ],
            ..Said::default()
        }
    );
    let r = o.readout;
    assert!(point_values(&r).iter().all(Option::is_none), "{r:#?}");
    assert_eq!(
        shared_values(&r),
        [Some("2.27E+04"), Some("24.0"), Some("2,800")]
    );
}

#[test]
fn unreadable_conditions_are_named_with_their_reasons() {
    let o = outcome(|i| {
        i.set(SlabField::Fc, "");
        i.set(SlabField::FinishLoad, "-1");
        i.set(SlabField::LiveLoad, "0");
        i.set(SlabField::Amplification, "0");
        i.set(SlabField::DeflectionRatio, "x");
        i.set(SlabField::DeflectionAbsolute, "-20");
    });
    // 読めない欄があるときはクライテリアが「無い」と言わない（欄をまたぐ理由が無い）
    assert_eq!(
        said(&o.record),
        Said {
            reasons: vec![
                (SlabField::Fc, "値を入れる"),
                (SlabField::FinishLoad, "0 以上の値を入れる"),
                (SlabField::Amplification, "0 より大きい値を入れる"),
                (SlabField::DeflectionRatio, "数値で入れる"),
                (SlabField::DeflectionAbsolute, "0 より大きい値を入れる"),
            ],
            ..Said::default()
        }
    );
    let r = o.readout;
    assert!(shared_values(&r).iter().all(Option::is_none));
    assert!(point_values(&r).iter().all(Option::is_none));
}

#[test]
fn fc_above_36_uses_the_heavier_rc_unit_weight_and_warns_outside_the_range() {
    let o = outcome(|i| i.set(SlabField::Fc, "42"));
    assert_eq!(o.readout.shared.rc_unit_weight.as_deref(), Some("24.5"));
    assert_eq!(said(&o.record), Said::default());
    let o = outcome(|i| i.set(SlabField::Fc, "120"));
    assert_eq!(
        said(&o.record),
        Said {
            warnings: vec![
                "Fc が単位体積重量の表の上限 100 N/mm² を超える。γ は最後の区分の値を使う"
            ],
            ..Said::default()
        }
    );
    assert!(o.readout.point.required_thickness.is_some());
}

// 等厚曲線の図（Issue #75）

fn figure(change: impl FnOnce(&mut SlabInputs)) -> Option<SlabFigure> {
    let mut inputs = SlabInputs::default();
    change(&mut inputs);
    let curves = SlabOutcome::from_inputs(&inputs).curves?;
    Some(SlabFigure::new(&curves))
}

/// 図の文字（白抜きの数値、目盛りの数値、軸の名前、条件の要約、検討点の名前）
fn texts(scene: &Scene) -> Vec<&str> {
    scene
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// 等厚曲線の上の数値（白抜きの等幅の文字。条件の要約と検討点の名前は和文の書体）
fn thickness_numbers(scene: &Scene) -> Vec<f64> {
    scene
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text {
                text,
                font: Font::Mono,
                halo: Some(_),
                ..
            } => Some(text.replace(',', "").parse().expect("数値")),
            _ => None,
        })
        .collect()
}

/// 検討点の印の中心
fn marks(scene: &Scene) -> Vec<Pt> {
    scene
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Circle { center, .. } => Some(*center),
            _ => None,
        })
        .collect()
}

#[test]
fn the_page_shows_the_iso_thickness_figure_after_the_results() {
    let main = element(&render_path("/tools/slab-thickness"), "main").to_string();
    assert_in_order(
        &text_of(&main),
        &["たわみ（mm）", "許容", "必要スラブ厚の等厚曲線（mm）"],
    );
    // 図は 1 枚（ライトとダークは CSS 変数で切り替わる）
    assert_eq!(main.matches("<svg xmlns").count(), 1, "{main}");
    assert!(main.contains("role=\"img\""), "{main}");
}

#[test]
fn the_curves_around_the_point_are_numbered_every_10_mm_and_bracket_its_thickness() {
    let figure = figure(|_| {}).expect("初期値で図が出る");
    let t_req: f64 = readout(|_| {})
        .point
        .required_thickness
        .expect("初期値で解ける")
        .parse()
        .unwrap();
    let numbers = thickness_numbers(&figure.scene);
    assert!(!numbers.is_empty(), "{:?}", figure.scene);
    assert!(
        numbers.iter().all(|t| t % 10.0 == 0.0 && *t >= 80.0),
        "{numbers:?}"
    );
    assert!(numbers.iter().any(|t| *t < t_req), "{numbers:?} / {t_req}");
    assert!(numbers.iter().any(|t| *t > t_req), "{numbers:?} / {t_req}");
}

#[test]
fn the_initial_view_is_a_square_centered_on_the_point() {
    // 検討点 (4.0, 5.0) が 1 つ: 幅 2 m の正方形。目盛りは 0.5 m おき
    let initial = figure(|_| {}).unwrap();
    let texts = texts(&initial.scene);
    for t in [
        "3.0",
        "3.5",
        "4.0",
        "4.5",
        "5.0",
        "5.5",
        "6.0",
        "Lx（m）",
        "Ly（m）",
    ] {
        assert!(texts.contains(&t), "{t} が無い: {texts:?}");
    }
    assert!(
        !texts.contains(&"2.5") && !texts.contains(&"6.5"),
        "{texts:?}"
    );
    let wide = figure(|i| {
        i.set(SlabField::SpanX, "8");
        i.set(SlabField::SpanY, "8");
    })
    .unwrap();
    let texts = self::texts(&wide.scene);
    for t in ["7.0", "9.0"] {
        assert!(texts.contains(&t), "{t} が無い: {texts:?}");
    }
}

#[test]
fn the_figure_holds_the_framed_view_as_its_plot_and_its_ticks_and_names_hit_nothing() {
    // 検討点 (4.0, 5.0) が 1 つ: 範囲は 3〜5 m × 4〜6 m
    let scene = figure(|_| {}).unwrap().scene;
    assert_eq!(scene.plots.len(), 1, "{:?}", scene.plots);
    let (x, y, side) = frame(&scene);
    assert_eq!(scene.plots[0].bounds(), [[x, y], [x + side, y + side]]);
    // 枠の 4 隅と中心（枠の線の上も当たる）
    for (screen, data) in [
        ([x, y + side], [3.0, 4.0]),
        ([x + side, y + side], [5.0, 4.0]),
        ([x, y], [3.0, 6.0]),
        ([x + side, y], [5.0, 6.0]),
        ([x + side / 2.0, y + side / 2.0], [4.0, 5.0]),
    ] {
        let (k, p) = scene.locate(screen).expect("枠の中");
        assert_eq!(k, 0);
        assert!(
            (p[0] - data[0]).abs() < 1e-12 && (p[1] - data[1]).abs() < 1e-12,
            "{screen:?} → {p:?}"
        );
    }
    // 枠の外の目盛りの数値と軸の名前は当たらない
    let outside: Vec<(&str, Pt)> = scene
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text {
                x: tx, y: ty, text, ..
            } if *tx < x || *ty > y + side => Some((text.as_str(), [*tx, *ty])),
            _ => None,
        })
        .collect();
    for name in ["3.0", "6.0", "Lx（m）", "Ly（m）"] {
        assert!(
            outside.iter().any(|(t, _)| *t == name),
            "{name}: {outside:?}"
        );
    }
    for (text, at) in outside {
        assert_eq!(scene.locate(at), None, "{text}");
    }
}

#[test]
fn the_point_is_marked_with_its_name() {
    let scene = figure(|i| i.set(SlabField::Name, "S2")).unwrap().scene;
    assert_eq!(marks(&scene).len(), 1, "{scene:?}");
    assert!(texts(&scene).contains(&"S2"), "{scene:?}");
}

#[test]
fn the_summary_lists_the_conditions_and_only_the_active_criteria() {
    let texts = texts(&figure(|_| {}).unwrap().scene).join("\n");
    for t in ["Fc=24", "wp=2,800", "α=16", "δ/lx≤1/400"] {
        assert!(texts.contains(t), "{t} が無い: {texts}");
    }
    assert!(
        !texts.contains("δ≤") && !texts.contains("表18-1"),
        "{texts}"
    );

    let scene = figure(|i| {
        i.set(SlabField::DeflectionRatio, "");
        i.set(SlabField::DeflectionAbsolute, "20");
        i.set(SlabField::MinThickness, "true");
    })
    .unwrap()
    .scene;
    let texts = self::texts(&scene).join("\n");
    assert!(
        texts.contains("δ≤20") && texts.contains("表18-1"),
        "{texts}"
    );
    assert!(!texts.contains("δ/lx"), "{texts}");
}

#[test]
fn no_figure_while_the_conditions_are_unreadable_or_without_criteria() {
    assert_eq!(figure(|i| i.set(SlabField::Fc, "")), None);
    assert_eq!(figure(|i| i.set(SlabField::Amplification, "0")), None);
    assert_eq!(figure(|i| i.set(SlabField::DeflectionRatio, "")), None);
    let html = render_path("/tools/slab-thickness");
    assert!(!html.contains("図は描かない"), "初期値で注記が出ている");
}

#[test]
fn an_unreadable_point_leaves_the_default_view_without_a_mark() {
    let scene = figure(|i| i.set(SlabField::SpanX, "abc"))
        .expect("条件が読めれば図は出る")
        .scene;
    assert!(marks(&scene).is_empty(), "{scene:?}");
    // 検討点が無いときの範囲は両軸 3〜9 m
    let texts = texts(&scene);
    for t in ["4", "6", "8"] {
        assert!(texts.contains(&t), "{t} が無い: {texts:?}");
    }
}

/// 塗った帯の小片（塗りのある閉じた折れ線）。画面の点の列
fn bands(scene: &Scene) -> Vec<&[Pt]> {
    scene
        .items
        .iter()
        .flat_map(|item| match item {
            Item::Path {
                subpaths,
                closed: true,
                fill: Some(_),
                ..
            } => subpaths.iter().map(Vec::as_slice).collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// 枠（最初の矩形）の左上と一辺
fn frame(scene: &Scene) -> (f64, f64, f64) {
    scene
        .items
        .iter()
        .find_map(|item| match item {
            Item::Rect { x, y, width, .. } => Some((*x, *y, *width)),
            _ => None,
        })
        .expect("枠がある")
}

/// 点が多角形の内にあるか（偶奇の規則）
fn inside(polygon: &[Pt], [x, y]: Pt) -> bool {
    let mut odd = false;
    for (i, &[xa, ya]) in polygon.iter().enumerate() {
        let [xb, yb] = polygon[(i + 1) % polygon.len()];
        if (ya > y) != (yb > y) && x < xa + (y - ya) / (yb - ya) * (xb - xa) {
            odd = !odd;
        }
    }
    odd
}

#[test]
fn curves_under_80_mm_are_drawn_and_numbered_but_their_regions_are_left_unpainted() {
    // 2.5 m 四方: 範囲（1.5〜3.5 m）の左下は 80 mm に届かず、右上は超える
    let scene = figure(|i| {
        i.set(SlabField::SpanX, "2.5");
        i.set(SlabField::SpanY, "2.5");
    })
    .unwrap()
    .scene;
    let numbers = thickness_numbers(&scene);
    assert!(numbers.contains(&80.0), "{numbers:?}");
    assert!(numbers.contains(&70.0), "{numbers:?}");
    // 枠の左下の隅（80 mm 未満）はどの帯にも入らない。80 mm の曲線の上の右上の隅は入る
    let (x, y, side) = frame(&scene);
    let bands = bands(&scene);
    assert!(!bands.is_empty());
    assert!(
        !bands.iter().any(|b| inside(b, [x + 1.0, y + side - 1.0])),
        "左下を塗った"
    );
    assert!(bands.iter().any(|b| inside(b, [x + side - 1.0, y + 1.0])));
}

#[test]
fn curve_numbers_keep_off_the_point_mark_and_its_name() {
    // 検討点はいつも範囲の中心、数値を並べる対角線の上にある。どの厚さに近くても印と名前を避ける
    for k in 0..12 {
        let lx = format!("{:.2}", 4.0 + 0.05 * k as f64);
        let scene = figure(|i| i.set(SlabField::SpanX, lx.as_str()))
            .unwrap()
            .scene;
        let [[cx, cy]] = marks(&scene)[..] else {
            panic!("印が 1 つでない: {scene:?}")
        };
        for item in &scene.items {
            // 等厚曲線の数値（白抜きの等幅の文字）
            let Item::Text {
                x,
                y,
                font: Font::Mono,
                halo: Some(_),
                ..
            } = *item
            else {
                continue;
            };
            // 印の周り: 中心どうしが、印の半径と数値の文字の高さの半分より近い（名前は印の右上）
            let near_mark = (x - cx).hypot(y - cy) < 10.0;
            let on_name = x > cx && x < cx + 30.0 && y < cy && y > cy - 20.0;
            assert!(
                !near_mark && !on_name,
                "Lx {lx}: 数値 ({x}, {y}) が印 ({cx}, {cy}) に重なる"
            );
        }
    }
}

#[test]
fn the_color_edge_curves_around_the_point_are_numbered_even_where_the_mark_sits_on_the_diagonal() {
    // 検討点は数値を並べる対角線の上にある。印と名前にかかる曲線も、同じ曲線の別の場所に数値が出る
    for k in 0..12 {
        let lx = format!("{:.2}", 4.0 + 0.05 * k as f64);
        let scene = figure(|i| i.set(SlabField::SpanX, lx.as_str()))
            .unwrap()
            .scene;
        let t_req: f64 = readout(|i| i.set(SlabField::SpanX, lx.as_str()))
            .point
            .required_thickness
            .expect("解ける")
            .parse()
            .unwrap();
        let numbers = thickness_numbers(&scene);
        let below = (t_req / 20.0).floor() * 20.0;
        for t in [below, below + 20.0] {
            assert!(numbers.contains(&t), "Lx {lx}: {t} が無い {numbers:?}");
        }
    }
}

#[test]
fn an_unreadable_criterion_is_not_called_missing() {
    // 読めないクライテリアは入れてはあるので、欄をまたぐ理由は無い
    let o = outcome(|i| {
        i.set(SlabField::DeflectionRatio, "");
        i.set(SlabField::DeflectionAbsolute, "x");
    });
    assert_eq!(
        said(&o.record),
        Said {
            reasons: vec![(SlabField::DeflectionAbsolute, "数値で入れる")],
            ..Said::default()
        }
    );
}

// 入力の組の HTML（`scripts/screenshot.sh` は欄に打てないので、打った後の状態はここで見る）

#[test]
fn the_initial_groups_say_nothing() {
    let [conditions, point] = groups(|_| {});
    assert_eq!(
        paragraphs(&conditions),
        Vec::<String>::new(),
        "{conditions}"
    );
    // 検討点の組の後ろは図の列で、段落は点の値の行（読み方の文）だけ
    assert_eq!(paragraphs(&point), [HINT], "{point}");
    for group in [conditions, point] {
        assert_eq!(flagged_heads(&group), 0, "{group}");
    }
}

#[test]
fn no_criteria_is_said_once_under_the_conditions_and_flags_the_three_criteria() {
    let [conditions, point] = groups(|i| i.set(SlabField::DeflectionRatio, ""));
    assert_eq!(paragraphs(&conditions), ["クライテリアを 1 つ以上入れる"]);
    assert_eq!(flagged_heads(&conditions), 3, "{conditions}");
    assert_eq!(paragraphs(&point), Vec::<String>::new(), "{point}");
    assert_eq!(flagged_heads(&point), 0, "{point}");
}

#[test]
fn unreadable_fields_are_said_under_the_group_of_their_sheet() {
    let [conditions, point] = groups(|i| {
        i.set(SlabField::Fc, "0");
        i.set(SlabField::Amplification, "x");
        i.set(SlabField::SpanY, "");
    });
    assert_eq!(
        paragraphs(&conditions),
        [
            "Fc（N/mm²）: 0 より大きい値を入れる",
            "変形増大係数: 数値で入れる",
        ]
    );
    assert_eq!(flagged_heads(&conditions), 2, "{conditions}");
    assert_eq!(paragraphs(&point), ["Ly（m）: 値を入れる"]);
    assert_eq!(flagged_heads(&point), 1, "{point}");
}

#[test]
fn the_fc_warning_is_said_under_the_conditions_only() {
    let [conditions, point] = groups(|i| i.set(SlabField::Fc, "12"));
    assert_eq!(paragraphs(&conditions), ["! Fc が下限 18 N/mm² に満たない"]);
    assert_eq!(flagged_heads(&conditions), 0, "{conditions}");
    // 検討点の組の後ろは図の列で、段落は点の値の行（読み方の文）だけ
    assert_eq!(paragraphs(&point), [HINT], "{point}");
}

// 点の値（Issue #139）

/// 初期値の条件（Fc 24、仕上げ 1000 N/m²、積載 1800 N/m²、変形増大係数 16、変形角 1/400）を、計算核で
/// 直接 Lx・Ly（m）で計算した必要スラブ厚。結果の表と同じ小数 1 桁の文字。値が出なければ無し
fn core_thickness(x: f64, y: f64) -> Option<String> {
    let conditions = Conditions {
        concrete: Concrete::new(Stress::n_per_mm2(24.0), ConcreteKind::Normal).unwrap(),
        finish_load: AreaLoad::n_per_m2(1000.0),
        live_load: AreaLoad::n_per_m2(1800.0),
        amplification: 16.0,
        criteria: CriteriaSet {
            deflection_ratio: Some(1.0 / 400.0),
            deflection_absolute: None,
            min_thickness: false,
        },
    };
    check(&conditions)
        .unwrap()
        .evaluate(Spans {
            x: Length::m(x),
            y: Length::m(y),
        })
        .ok()
        .map(|ev| fixed(ev.required_thickness.as_mm(), 1))
}

/// 点の値の項を「Lx = 4.00 m」の形の文字に（値が無ければ「—」）
fn terms_text(terms: &[PointTerm]) -> Vec<String> {
    terms
        .iter()
        .map(|t| {
            let value = t.value.clone().unwrap_or_else(|| "—".into());
            format!("{} = {value} {}", t.symbol, t.unit)
        })
        .collect()
}

/// 図の描く範囲の、左から `fx`、下から `fy` の割合の画面の点
fn screen_at(scene: &Scene, fx: f64, fy: f64) -> Pt {
    let [lo, hi] = scene.plots[0].bounds();
    [lo[0] + fx * (hi[0] - lo[0]), hi[1] - fy * (hi[1] - lo[1])]
}

/// 図の画面の点 `screen` に止めた、指した点
fn pinned_at(scene: &Scene, screen: Pt) -> Pointer {
    let (plot, at) = scene.locate(screen).expect("描く範囲の中");
    Pointer::Pinned(Pointed { plot, at })
}

#[test]
fn the_point_value_is_the_required_thickness_of_the_core_at_that_lx_and_ly() {
    let curves = outcome(|_| {}).curves.expect("初期値で図が出る");
    for [x, y] in [[4.0, 5.0], [3.0, 4.0], [3.37, 5.82], [5.0, 6.0], [4.6, 4.1]] {
        assert_eq!(
            terms_text(&curves.point_value([x, y])),
            [
                format!("Lx = {} m", fixed(x, 2)),
                format!("Ly = {} m", fixed(y, 2)),
                format!("t_req = {} mm", core_thickness(x, y).unwrap()),
            ],
            "{x} {y}"
        );
    }
    // 検討点の値は、結果の表の必要スラブ厚と同じ文字
    assert_eq!(
        curves.point_value([4.0, 5.0])[2].value,
        readout(|_| {}).point.required_thickness
    );
    // 図の上の点から当てても同じ（範囲は 3〜5 m × 4〜6 m。左から 1/4、下から 3/4 は Lx 3.5 m、Ly 5.5 m）
    let scene = figure(|_| {}).unwrap().scene;
    let at = pinned_at(&scene, screen_at(&scene, 0.25, 0.75))
        .point()
        .unwrap()
        .at;
    assert_eq!(
        terms_text(&curves.point_value(at)),
        [
            "Lx = 3.50 m".to_string(),
            "Ly = 5.50 m".into(),
            format!("t_req = {} mm", core_thickness(3.5, 5.5).unwrap()),
        ]
    );
}

#[test]
fn a_point_where_the_core_gives_no_value_reads_a_dash() {
    // 0.3 m 四方: 範囲は -0.7〜1.3 m で、0 以下のスパンを含む
    let change = |i: &mut SlabInputs| {
        i.set(SlabField::SpanX, "0.3");
        i.set(SlabField::SpanY, "0.3");
    };
    let scene = figure(change).expect("小さなスパンでも図は出る").scene;
    let curves = outcome(change).curves.unwrap();
    for screen in [screen_at(&scene, 0.1, 0.6), screen_at(&scene, 0.6, 0.1)] {
        let at = pinned_at(&scene, screen).point().unwrap().at;
        assert!(at[0] <= 0.0 || at[1] <= 0.0, "{at:?}");
        assert_eq!(core_thickness(at[0], at[1]), None);
        let terms = curves.point_value(at);
        assert_eq!(terms[2].value, None, "{at:?}");
        assert_eq!(terms_text(&terms)[2], "t_req = — mm");
        // Lx・Ly は出す
        assert!(terms[0].value.is_some() && terms[1].value.is_some());
    }
    let at = pinned_at(&scene, screen_at(&scene, 0.6, 0.6))
        .point()
        .unwrap()
        .at;
    assert_eq!(
        curves.point_value(at)[2].value,
        core_thickness(at[0], at[1])
    );
    assert!(curves.point_value(at)[2].value.is_some());
}

#[test]
fn a_pinned_point_is_kept_across_the_inputs_and_cleared_when_the_view_moves_away() {
    let scene = figure(|_| {}).unwrap().scene;
    // 範囲は 3〜5 m × 4〜6 m。左から 3/4、下から 1/4 は Lx 4.5 m、Ly 4.5 m
    let pinned = pinned_at(&scene, screen_at(&scene, 0.75, 0.25));
    let at = pinned.point().unwrap().at;
    assert!(
        (at[0] - 4.5).abs() < 1e-9 && (at[1] - 4.5).abs() < 1e-9,
        "{at:?}"
    );
    let initial = outcome(|_| {}).curves.unwrap().point_value(at);

    // 条件を変えても範囲は同じで、同じ点の値を出し直す
    let heavier = |i: &mut SlabInputs| i.set(SlabField::LiveLoad, "3600");
    assert_eq!(pinned.within(&figure(heavier).unwrap().scene), pinned);
    let again = outcome(heavier).curves.unwrap().point_value(at);
    assert_eq!(terms_text(&again[..2]), ["Lx = 4.50 m", "Ly = 4.50 m"]);
    assert_ne!(again[2], initial[2]);

    // 検討点を少し動かすと範囲も動くが、点が中にあれば残る（範囲は 3.2〜5.2 m × 4〜6 m）
    let nudged = |i: &mut SlabInputs| i.set(SlabField::SpanX, "4.2");
    assert_eq!(pinned.within(&figure(nudged).unwrap().scene), pinned);

    // 検討点が動いて範囲が 7〜9 m 四方になると、点は外に出て消える
    let moved = figure(|i| {
        i.set(SlabField::SpanX, "8");
        i.set(SlabField::SpanY, "8");
    })
    .unwrap()
    .scene;
    assert_eq!(pinned.within(&moved), Pointer::Nothing);
}

#[test]
fn the_page_says_how_to_point_between_the_figure_and_its_notes() {
    // 図を描く前（SSG）は読み方の文。十字線の層は無く、図は読み上げの名前の付いた img
    let main = element(&render_path("/tools/slab-thickness"), "main").to_string();
    assert_in_order(
        &text_of(&main),
        &[
            "必要スラブ厚の等厚曲線（mm）",
            HINT,
            "曲線は 10 mm ごと、色は 20 mm ごと",
        ],
    );
    assert_eq!(count_tags(&main, "figure", &[("role", "img")]), 1, "{main}");
    assert_eq!(main.matches("<svg xmlns").count(), 1, "{main}");
    // 条件が読めず図を描かないときは、読み方の文も無い
    for change in [
        (|i: &mut SlabInputs| i.set(SlabField::Fc, "")) as fn(&mut SlabInputs),
        |i| i.set(SlabField::DeflectionRatio, ""),
    ] {
        let main = element(
            &render_path_with("/tools/slab-thickness", inputs(change)),
            "main",
        )
        .to_string();
        assert!(main.contains("条件が読めるまで図は描かない"), "{main}");
        assert!(!main.contains(HINT), "{main}");
    }
}
