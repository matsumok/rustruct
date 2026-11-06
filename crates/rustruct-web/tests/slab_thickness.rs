//! 必要スラブ厚さ（Issue #17）: `/tools` の一覧と `/tools/slab-thickness` のページ。
//! 言葉は `docs/tools/slab-thickness/GLOSSARY.md`。数値は `rustruct-calc` が出し、ここでは表示の文字列を見る。

mod common;
use common::{assert_in_order, element, reasons, text_of};
use rustruct_web::{SlabField, SlabFigure, SlabInputs, SlabOutcome, SlabReadout, render_path};

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

fn readout(change: impl FnOnce(&mut SlabInputs)) -> SlabReadout {
    let mut inputs = SlabInputs::default();
    change(&mut inputs);
    SlabOutcome::from_inputs(&inputs).readout
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
    let r = readout(|_| {});
    assert!(r.errors.is_empty() && !r.no_criteria && r.warning.is_none());
    let p = &r.point;
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
    let r = readout(|i| {
        i.set(SlabField::DeflectionRatio, "");
        i.set(SlabField::MinThickness, "true");
    });
    assert!(r.errors.is_empty() && !r.no_criteria);
    // wp = 2800 N/m²、lx = 4000 mm、λ = 1.25: 0.02·(0.55/0.65)·(1+0.28+0.4)·4000 = 113.7
    assert_eq!(r.point.required_thickness.as_deref(), Some("113.7"));
    assert_eq!(r.point.governing.as_deref(), Some("表18-1"));
    // たわみに効く条件が無いので、許容たわみは無い
    assert_eq!(r.point.allowable_deflection, None);
}

#[test]
fn blank_criteria_with_the_table_off_is_an_error_and_nothing_is_evaluated() {
    let r = readout(|i| i.set(SlabField::DeflectionRatio, ""));
    assert!(r.no_criteria);
    assert!(r.errors.is_empty());
    assert!(point_values(&r).iter().all(Option::is_none), "{r:#?}");
    // 共有値は条件だけで決まるので出る
    assert_eq!(
        shared_values(&r),
        [Some("2.27E+04"), Some("24.0"), Some("2,800")]
    );
}

#[test]
fn an_unreadable_span_leaves_the_shared_values_and_dashes_the_point() {
    let r = readout(|i| {
        i.set(SlabField::SpanX, "0");
        i.set(SlabField::SpanY, "abc");
    });
    assert_eq!(
        reasons(&r.errors),
        [
            (SlabField::SpanX, "0 より大きい値を入れる"),
            (SlabField::SpanY, "数値で入れる"),
        ]
    );
    assert!(point_values(&r).iter().all(Option::is_none), "{r:#?}");
    assert_eq!(
        shared_values(&r),
        [Some("2.27E+04"), Some("24.0"), Some("2,800")]
    );
}

#[test]
fn unreadable_conditions_are_named_with_their_reasons() {
    let r = readout(|i| {
        i.set(SlabField::Fc, "");
        i.set(SlabField::FinishLoad, "-1");
        i.set(SlabField::LiveLoad, "0");
        i.set(SlabField::Amplification, "0");
        i.set(SlabField::DeflectionRatio, "x");
        i.set(SlabField::DeflectionAbsolute, "-20");
    });
    assert_eq!(
        reasons(&r.errors),
        [
            (SlabField::Fc, "値を入れる"),
            (SlabField::FinishLoad, "0 以上の値を入れる"),
            (SlabField::Amplification, "0 より大きい値を入れる"),
            (SlabField::DeflectionRatio, "数値で入れる"),
            (SlabField::DeflectionAbsolute, "0 より大きい値を入れる"),
        ]
    );
    assert!(!r.no_criteria, "読めない欄があるときは「無い」と言わない");
    assert!(shared_values(&r).iter().all(Option::is_none));
    assert!(point_values(&r).iter().all(Option::is_none));
}

#[test]
fn fc_above_36_uses_the_heavier_rc_unit_weight_and_warns_outside_the_range() {
    let r = readout(|i| i.set(SlabField::Fc, "42"));
    assert_eq!(r.shared.rc_unit_weight.as_deref(), Some("24.5"));
    assert_eq!(r.warning, None);
    let r = readout(|i| i.set(SlabField::Fc, "120"));
    assert!(r.warning.is_some(), "{r:#?}");
    assert!(r.point.required_thickness.is_some());
}

// 等厚曲線の図（Issue #75）

fn figure(change: impl FnOnce(&mut SlabInputs)) -> Option<SlabFigure> {
    let mut inputs = SlabInputs::default();
    change(&mut inputs);
    let curves = SlabOutcome::from_inputs(&inputs).curves?;
    Some(SlabFigure::new(&curves))
}

/// SVG の文字（白抜きの数値、目盛りの数値、軸の名前、条件の要約、検討点の名前）
fn svg_texts(svg: &str) -> Vec<String> {
    svg.split("<text")
        .skip(1)
        .map(|t| t[t.find('>').unwrap() + 1..t.find("</text>").unwrap()].to_string())
        .collect()
}

/// 等厚曲線の上の数値（白抜きの等幅の文字）
fn thickness_numbers(svg: &str) -> Vec<f64> {
    svg.split("<text")
        .skip(1)
        .filter(|t| t[..t.find('>').unwrap()].contains("paint-order"))
        .filter_map(|t| {
            t[t.find('>').unwrap() + 1..t.find("</text>").unwrap()]
                .replace(',', "")
                .parse()
                .ok()
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
    // ライトとダークの 2 枚
    assert_eq!(main.matches("<svg xmlns").count(), 2, "{main}");
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
    let numbers = thickness_numbers(&figure.light_svg);
    assert!(!numbers.is_empty(), "{}", figure.light_svg);
    assert!(
        numbers.iter().all(|t| t % 10.0 == 0.0 && *t >= 80.0),
        "{numbers:?}"
    );
    assert!(numbers.iter().any(|t| *t < t_req), "{numbers:?} / {t_req}");
    assert!(numbers.iter().any(|t| *t > t_req), "{numbers:?} / {t_req}");
    assert_eq!(thickness_numbers(&figure.dark_svg), numbers);
}

#[test]
fn the_initial_view_is_a_square_centered_on_the_point() {
    // 検討点 (4.0, 5.0) が 1 つ: 幅 2 m の正方形。目盛りは 0.5 m おき
    let texts = svg_texts(&figure(|_| {}).unwrap().light_svg);
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
        assert!(texts.iter().any(|s| s == t), "{t} が無い: {texts:?}");
    }
    assert!(!texts.iter().any(|s| s == "2.5" || s == "6.5"), "{texts:?}");
    let texts = svg_texts(
        &figure(|i| {
            i.set(SlabField::SpanX, "8");
            i.set(SlabField::SpanY, "8");
        })
        .unwrap()
        .light_svg,
    );
    for t in ["7.0", "9.0"] {
        assert!(texts.iter().any(|s| s == t), "{t} が無い: {texts:?}");
    }
}

#[test]
fn the_point_is_marked_with_its_name() {
    let svg = figure(|i| i.set(SlabField::Name, "S2")).unwrap().light_svg;
    assert_eq!(svg.matches("<circle").count(), 1, "{svg}");
    assert!(svg_texts(&svg).iter().any(|s| s == "S2"), "{svg}");
}

#[test]
fn the_summary_lists_the_conditions_and_only_the_active_criteria() {
    let texts = svg_texts(&figure(|_| {}).unwrap().light_svg).join("\n");
    for t in ["Fc=24", "wp=2,800", "α=16", "δ/lx≤1/400"] {
        assert!(texts.contains(t), "{t} が無い: {texts}");
    }
    assert!(
        !texts.contains("δ≤") && !texts.contains("表18-1"),
        "{texts}"
    );

    let texts = svg_texts(
        &figure(|i| {
            i.set(SlabField::DeflectionRatio, "");
            i.set(SlabField::DeflectionAbsolute, "20");
            i.set(SlabField::MinThickness, "true");
        })
        .unwrap()
        .light_svg,
    )
    .join("\n");
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
    let svg = figure(|i| i.set(SlabField::SpanX, "abc"))
        .expect("条件が読めれば図は出る")
        .light_svg;
    assert!(!svg.contains("<circle"), "{svg}");
    // 検討点が無いときの範囲は両軸 3〜9 m
    let texts = svg_texts(&svg);
    for t in ["4", "6", "8"] {
        assert!(texts.iter().any(|s| s == t), "{t} が無い: {texts:?}");
    }
}

#[test]
fn regions_under_80_mm_are_left_unpainted_and_no_curve_is_thinner() {
    // 2.5 m 四方: 範囲（1.5〜3.5 m）の左下は 80 mm に届かず、右上は超える
    let svg = figure(|i| {
        i.set(SlabField::SpanX, "2.5");
        i.set(SlabField::SpanY, "2.5");
    })
    .unwrap()
    .light_svg;
    let numbers = thickness_numbers(&svg);
    assert!(numbers.contains(&80.0), "{numbers:?}");
    assert!(numbers.iter().all(|t| *t >= 80.0), "{numbers:?}");
}

/// 属性の値（最初の 1 つ）
fn attr(tag: &str, name: &str) -> f64 {
    let at = tag.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
    tag[at..at + tag[at..].find('"').unwrap()].parse().unwrap()
}

#[test]
fn curve_numbers_keep_off_the_point_mark_and_its_name() {
    // 検討点はいつも範囲の中心、数値を並べる対角線の上にある。どの厚さに近くても印と名前を避ける
    for k in 0..12 {
        let lx = format!("{:.2}", 4.0 + 0.05 * k as f64);
        let svg = figure(|i| i.set(SlabField::SpanX, lx.as_str()))
            .unwrap()
            .light_svg;
        let circle = &svg[svg.find("<circle").unwrap()..];
        let (cx, cy) = (attr(circle, "cx"), attr(circle, "cy"));
        for t in svg.split("<text").skip(1) {
            let tag = &t[..t.find('>').unwrap()];
            if !(tag.contains("paint-order") && tag.contains("font-size=\"9\"")) {
                continue;
            }
            let (x, y) = (attr(tag, "x"), attr(tag, "y"));
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
        let svg = figure(|i| i.set(SlabField::SpanX, lx.as_str()))
            .unwrap()
            .light_svg;
        let t_req: f64 = readout(|i| i.set(SlabField::SpanX, lx.as_str()))
            .point
            .required_thickness
            .expect("解ける")
            .parse()
            .unwrap();
        let numbers = thickness_numbers(&svg);
        let below = (t_req / 20.0).floor() * 20.0;
        for t in [below, below + 20.0] {
            assert!(numbers.contains(&t), "Lx {lx}: {t} が無い {numbers:?}");
        }
    }
}

#[test]
fn an_unreadable_criterion_is_not_called_missing() {
    let r = readout(|i| {
        i.set(SlabField::DeflectionRatio, "");
        i.set(SlabField::DeflectionAbsolute, "x");
    });
    assert_eq!(
        reasons(&r.errors),
        [(SlabField::DeflectionAbsolute, "数値で入れる")]
    );
    assert!(!r.no_criteria, "読めないクライテリアは入れてはある");
}
