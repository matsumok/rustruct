//! Kirchhoff 板の曲げ（Issue #36）: `/tools` の一覧と `/tools/kirchhoff-plate` のページ。
//! 言葉は `docs/tools/kirchhoff-plate/GLOSSARY.md`。数値は `rustruct-calc` が出し、ここでは表示の文字列を見る。

mod common;
use common::{assert_in_order, element, text_of};
use rustruct_types::kirchhoff_plate::{Support, Supports};
use rustruct_web::{InputField, PlateInputs, PlateReadout, render_path};

#[test]
fn tools_lists_kirchhoff_plate_under_the_mechanics_heading() {
    let html = render_path("/tools");
    assert_in_order(
        &text_of(element(&html, "main")),
        &[
            "/tools",
            "Tools",
            "Mechanics",
            "/tools/kirchhoff-plate",
            "Kirchhoff 板の曲げ",
            "長方形の板",
        ],
    );
    assert!(
        html.contains("href=\"/tools/kirchhoff-plate\""),
        "Kirchhoff 板の曲げへのリンクが無い: {html}"
    );
}

#[test]
fn the_tool_page_is_headed_with_its_name_under_its_flat_address() {
    let html = render_path("/tools/kirchhoff-plate");
    assert!(html.contains("<title>Kirchhoff 板の曲げ · rustruct.matsumok.com</title>"));
    assert_in_order(
        &text_of(element(&html, "main")),
        &["/tools/kirchhoff-plate", "Kirchhoff 板の曲げ"],
    );
}

#[test]
fn the_page_opens_with_the_initial_inputs_in_the_sheet() {
    let html = render_path("/tools/kirchhoff-plate");
    let main = element(&html, "main");
    assert_in_order(
        &text_of(main),
        &[
            "Lx（m）",
            "Ly（m）",
            "t（mm）",
            "E（N/mm²）",
            "ν",
            "p（kN/m²）",
            "x = 0（左）",
            "x = Lx（右）",
            "y = 0（下）",
            "y = Ly（上）",
        ],
    );
    for value in ["4.0", "5.0", "150", "22700", "0", "10"] {
        assert!(
            main.contains(&format!("value=\"{value}\"")),
            "入力欄に初期値 {value} が無い: {main}"
        );
    }
    // 四辺とも固定が選ばれている
    assert_eq!(main.matches("selected").count(), 4, "{main}");
}

#[test]
fn the_page_shows_the_results_of_the_fixed_plate_in_order() {
    let text = text_of(element(&render_path("/tools/kirchhoff-plate"), "main"));
    assert_in_order(
        &text,
        &[
            "板の中央",
            "w（mm）",
            "Mx（kN·m/m）",
            "My（kN·m/m）",
            "辺の中点",
            "M（kN·m/m）",
            "V（kN/m）",
            "x = 0",
            "x = Lx",
            "y = 0",
            "y = Ly",
            "辺の最大値",
            "せん断力",
            "辺の反力",
            "隅反力",
            "x = 0・y = 0",
            "x = Lx・y = 0",
            "x = Lx・y = Ly",
            "x = 0・y = Ly",
            "釣り合い",
            "200",
            "200",
        ],
    );
    assert!(
        !text.contains('—'),
        "四辺固定で計算できない値がある: {text}"
    );
    assert!(!text.contains("入力"), "部の見出し「入力」がある: {text}");
    assert!(!text.contains("結果"), "部の見出し「結果」がある: {text}");
}

#[test]
fn the_signs_are_stated_in_the_notes() {
    let text = text_of(element(&render_path("/tools/kirchhoff-plate"), "main"));
    for note in [
        "たわみは下向きを正",
        "曲げモーメントは下側が引張になる向きを正",
        "辺の反力は支える向きを正",
        "隅反力は板を押さえる向きを正",
    ] {
        assert!(text.contains(note), "注記「{note}」が無い: {text}");
    }
}

fn readout(change: impl FnOnce(&mut PlateInputs)) -> PlateReadout {
    let mut inputs = PlateInputs::default();
    change(&mut inputs);
    PlateReadout::from_inputs(&inputs)
}

/// 結果の値がすべて「—」
fn assert_not_computed(r: &PlateReadout) {
    let c = &r.center;
    let mut values: Vec<&str> = [&c.w, &c.mx, &c.my].map(String::as_str).to_vec();
    for row in &r.edges {
        let (s, v) = (&row.max_shear, &row.max_reaction);
        values.extend(
            [
                &row.moment,
                &row.reaction,
                &s.value,
                &s.along,
                &v.value,
                &v.along,
            ]
            .map(String::as_str),
        );
    }
    values.extend(r.corners.iter().map(|(_, v)| v.as_str()));
    values.extend([&r.balance.reactions, &r.balance.load].map(String::as_str));
    assert!(values.iter().all(|v| *v == "—"), "{r:#?}");
}

#[test]
fn an_unstable_combination_shows_dashes_and_says_the_plate_is_not_held() {
    let r = readout(|i| i.supports = Supports::all(Support::Free));
    assert_eq!(r.notice, Some("この支持条件の組では板を支えられない"));
    assert!(r.errors.is_empty());
    assert_not_computed(&r);
}

#[test]
fn an_unsupported_combination_shows_dashes_and_says_it_cannot_be_solved_yet() {
    let r = readout(|i| i.supports.x_lx = Support::Free);
    assert_eq!(r.notice, Some("この組はまだ解けない"));
    assert_not_computed(&r);
}

#[test]
fn a_simply_supported_plate_balances_with_its_corner_reactions_pressing_down() {
    // 辺の反力の合計は荷重を超え、超えた分を角の隅反力が押さえる（kirchhoff-plate の ADR-0007）
    let r = readout(|i| i.supports = Supports::all(Support::SimplySupported));
    assert_eq!(r.notice, None);
    assert_eq!([&r.balance.reactions, &r.balance.load], ["200", "200"]);
    for (corner, reaction) in &r.corners {
        let value: f64 = reaction.parse().unwrap();
        assert!(
            value > 0.0,
            "{corner} の隅反力が押さえる向きでない: {reaction}"
        );
    }
}

#[test]
fn unreadable_fields_are_named_with_their_reasons_and_nothing_is_computed() {
    let r = readout(|i| {
        i.lx = "0".into();
        i.thickness = "-150".into();
        i.young_modulus = "abc".into();
        i.poisson_ratio = "0.5".into();
        i.load = String::new();
    });
    assert_eq!(
        r.errors,
        [
            (InputField::Lx, "0 より大きい値を入れる"),
            (InputField::Thickness, "0 より大きい値を入れる"),
            (InputField::YoungModulus, "数値で入れる"),
            (InputField::PoissonRatio, "0 以上 0.5 未満で入れる"),
            (InputField::Load, "値を入れる"),
        ]
    );
    assert_eq!(r.notice, None);
    assert_not_computed(&r);
}

#[test]
fn an_upward_load_is_computed_with_the_signs_turned() {
    let down = readout(|_| {});
    let up = readout(|i| i.load = "-10".into());
    assert_eq!(up.center.w, format!("-{}", down.center.w));
    assert_eq!([up.balance.reactions, up.balance.load], ["-200", "-200"]);
}

#[test]
fn a_swapped_plate_swaps_the_x_and_y_results() {
    let r = readout(|_| {});
    let swapped = readout(|i| (i.lx, i.ly) = ("5.0".into(), "4.0".into()));
    assert_eq!(swapped.center.mx, r.center.my);
    assert_eq!(swapped.edges[0].moment, r.edges[2].moment);
    assert_eq!(swapped.edges[2].reaction, r.edges[0].reaction);
}
