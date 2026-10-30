//! 必要スラブ厚さ（Issue #17）: `/tools` の一覧と `/tools/slab-thickness` のページ。
//! 言葉は `docs/tools/slab-thickness/GLOSSARY.md`。数値は `rustruct-calc` が出し、ここでは表示の文字列を見る。

mod common;
use common::{assert_in_order, element, text_of};
use rustruct_web::{PointRow, SharedRow, SlabField, SlabInputs, SlabReadout, render_path};

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
    // 表18-1 は初期オフ
    assert!(main.contains("type=\"checkbox\""), "{main}");
    assert!(!main.contains("checked"), "表18-1 がオンで開く: {main}");
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
            "22,669",
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
    SlabReadout::from_inputs(&inputs)
}

fn point_values(p: &PointRow) -> [&str; 7] {
    [
        &p.required_thickness,
        &p.governing,
        &p.short_span,
        &p.aspect_ratio,
        &p.elastic_deflection,
        &p.amplified_deflection,
        &p.allowable_deflection,
    ]
    .map(String::as_str)
}

fn shared_values(s: &SharedRow) -> [&str; 3] {
    [&s.young_modulus, &s.rc_unit_weight, &s.wp].map(String::as_str)
}

#[test]
fn the_initial_point_is_governed_by_the_deflection_ratio_at_its_limit() {
    let r = readout(|_| {});
    assert!(r.errors.is_empty() && !r.no_criteria && r.warning.is_none());
    let p = &r.point;
    assert_eq!(p.governing, "変形角");
    // 変形角が決めるので、増大後たわみ = 許容たわみ = 4000/400
    assert_eq!(p.amplified_deflection, "10.00");
    assert_eq!(p.allowable_deflection, "10.00");
}

#[test]
fn swapping_lx_and_ly_gives_the_same_results() {
    let swapped = readout(|i| (i.span_x, i.span_y) = ("5.0".into(), "4.0".into()));
    assert_eq!(swapped.point, readout(|_| {}).point);
}

#[test]
fn turning_on_the_table_and_blanking_the_ratio_leaves_only_the_minimum_thickness() {
    let r = readout(|i| {
        i.deflection_ratio = String::new();
        i.min_thickness = true;
    });
    assert!(r.errors.is_empty() && !r.no_criteria);
    // wp = 2800 N/m²、lx = 4000 mm、λ = 1.25: 0.02·(0.55/0.65)·(1+0.28+0.4)·4000 = 113.7
    assert_eq!(r.point.required_thickness, "113.7");
    assert_eq!(r.point.governing, "表18-1");
    // たわみに効く条件が無いので、許容たわみは無い
    assert_eq!(r.point.allowable_deflection, "—");
}

#[test]
fn blank_criteria_with_the_table_off_is_an_error_and_nothing_is_evaluated() {
    let r = readout(|i| i.deflection_ratio = String::new());
    assert!(r.no_criteria);
    assert!(r.errors.is_empty());
    assert!(point_values(&r.point).iter().all(|v| *v == "—"), "{r:#?}");
    // 共有値は条件だけで決まるので出る
    assert_eq!(shared_values(&r.shared), ["22,669", "24.0", "2,800"]);
}

#[test]
fn an_unreadable_span_leaves_the_shared_values_and_dashes_the_point() {
    let r = readout(|i| {
        i.span_x = "0".into();
        i.span_y = "abc".into();
    });
    assert_eq!(
        r.errors,
        [
            (SlabField::SpanX, "0 より大きい値を入れる"),
            (SlabField::SpanY, "数値で入れる"),
        ]
    );
    assert!(point_values(&r.point).iter().all(|v| *v == "—"), "{r:#?}");
    assert_eq!(shared_values(&r.shared), ["22,669", "24.0", "2,800"]);
}

#[test]
fn unreadable_conditions_are_named_with_their_reasons() {
    let r = readout(|i| {
        i.fc = String::new();
        i.finish_load = "-1".into();
        i.live_load = "0".into();
        i.amplification = "0".into();
        i.deflection_ratio = "x".into();
        i.deflection_absolute = "-20".into();
    });
    assert_eq!(
        r.errors,
        [
            (SlabField::Fc, "値を入れる"),
            (SlabField::FinishLoad, "0 以上の値を入れる"),
            (SlabField::Amplification, "0 より大きい値を入れる"),
            (SlabField::DeflectionRatio, "数値で入れる"),
            (SlabField::DeflectionAbsolute, "0 より大きい値を入れる"),
        ]
    );
    assert!(!r.no_criteria, "読めない欄があるときは「無い」と言わない");
    assert!(shared_values(&r.shared).iter().all(|v| *v == "—"));
    assert!(point_values(&r.point).iter().all(|v| *v == "—"));
}

#[test]
fn fc_above_36_uses_the_heavier_rc_unit_weight_and_warns_outside_the_range() {
    let r = readout(|i| i.fc = "42".into());
    assert_eq!(r.shared.rc_unit_weight, "24.5");
    assert_eq!(r.warning, None);
    let r = readout(|i| i.fc = "120".into());
    assert!(r.warning.is_some(), "{r:#?}");
    assert_ne!(r.point.required_thickness, "—");
}
