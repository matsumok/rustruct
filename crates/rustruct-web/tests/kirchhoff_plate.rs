//! Kirchhoff 板の曲げ（Issue #36）: `/tools` の一覧と `/tools/kirchhoff-plate` のページ。
//! 言葉は `docs/tools/kirchhoff-plate/GLOSSARY.md`。数値は `rustruct-calc` が出し、ここでは表示の文字列を見る。

mod common;
use common::{assert_in_order, element, reasons, text_of};
use rustruct_types::kirchhoff_plate::{Edge, Support};
use rustruct_web::{InputField, PlateFigure, PlateInputs, PlateReadout, Quantity, render_path};

#[test]
fn tools_lists_kirchhoff_plate_under_the_mechanics_heading() {
    let html = render_path("/tools");
    assert_in_order(
        &text_of(element(&html, "main")),
        &[
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
fn the_tool_page_is_headed_with_its_name() {
    // 住所（/tools/kirchhoff-plate）はヘッダーのパンくずで示す（render.rs）
    let html = render_path("/tools/kirchhoff-plate");
    assert!(html.contains("<title>Kirchhoff 板の曲げ · rustruct.matsumok.com</title>"));
    assert!(text_of(element(&html, "main")).starts_with("Kirchhoff 板の曲げ"));
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
    assert!(
        text_of(main).contains("ν は 0 を基本とする"),
        "ν の注記が無い: {main}"
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
            "最大たわみ",
            "w（mm）",
            "x（m）",
            "y（m）",
            "辺の最大値",
            "M（kN·m/m）",
            "Q（kN/m）",
            "V（kN/m）",
            "x = 0",
            "x = Lx",
            "y = 0",
            "y = Ly",
            "辺の最大値の位置（m）",
            "x = 0",
            "x = Lx",
            "y = 0",
            "y = Ly",
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

/// 4 辺とも同じ支持条件にする
fn set_all(inputs: &mut PlateInputs, support: Support) {
    for edge in Edge::ALL {
        inputs.set(InputField::Support(edge), support.symbol());
    }
}

/// 結果の値がすべて無し（「—」）
fn assert_not_computed(r: &PlateReadout) {
    let c = &r.center;
    let d = &r.deflection_maximum;
    let mut values: Vec<&Option<String>> = vec![&c.w, &c.mx, &c.my, &d.w, &d.x, &d.y];
    for row in &r.edges {
        for m in [&row.max_moment, &row.max_shear, &row.max_reaction] {
            values.extend([&m.value, &m.along]);
        }
    }
    values.extend(r.corners.iter().map(|(_, v)| v));
    values.extend([&r.balance.reactions, &r.balance.load]);
    assert!(values.iter().all(|v| v.is_none()), "{r:#?}");
}

#[test]
fn an_unstable_combination_shows_dashes_and_says_the_plate_is_not_held() {
    let r = readout(|i| set_all(i, Support::Free));
    assert_eq!(r.notice, Some("この支持条件の組では板を支えられない"));
    assert!(r.errors.is_empty());
    assert_not_computed(&r);
}

#[test]
fn a_combination_with_a_free_edge_is_solved() {
    // 不安定な組以外はすべて解ける（ADR kirchhoff-plate-0002）。釣り合いと自由辺の反力は計算核のテスト
    let r = readout(|i| i.set(InputField::Support(Edge::XLx), Support::Free.symbol()));
    assert_eq!(r.notice, None);
    assert!(r.errors.is_empty());
    assert!(r.balance.reactions.is_some());
    // 自由辺の反力は辺の全体で 0 なので、位置は無し
    let free = &r.edges[1].max_reaction;
    assert_eq!(
        [free.value.as_deref(), free.along.as_deref()],
        [Some("0"), None]
    );
}

#[test]
fn a_simply_supported_plate_shows_a_reaction_at_every_corner() {
    // 隅反力の向きと釣り合いは計算核のテスト（ADR kirchhoff-plate-0007）。ここでは隅ごとの行の名前と、
    // 値が出ていること
    let r = readout(|i| set_all(i, Support::SimplySupported));
    assert_eq!(r.notice, None);
    let names: Vec<&str> = r.corners.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "x = 0・y = 0",
            "x = Lx・y = 0",
            "x = Lx・y = Ly",
            "x = 0・y = Ly"
        ]
    );
    assert!(
        r.corners.iter().all(|(_, v)| v.is_some()),
        "{:?}",
        r.corners
    );
}

#[test]
fn unreadable_fields_are_named_with_their_reasons_and_nothing_is_computed() {
    let r = readout(|i| {
        i.set(InputField::Lx, "0");
        i.set(InputField::Thickness, "-150");
        i.set(InputField::YoungModulus, "abc");
        i.set(InputField::PoissonRatio, "0.5");
        i.set(InputField::Load, "");
    });
    assert_eq!(
        reasons(&r.errors),
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
    let up = readout(|i| i.set(InputField::Load, "-10"));
    let down_w = down.center.w.expect("下向きの荷重で解ける");
    assert_eq!(up.center.w, Some(format!("-{down_w}")));
    assert_eq!(
        [up.balance.reactions.as_deref(), up.balance.load.as_deref()],
        [Some("-200"); 2]
    );
}

#[test]
fn a_swapped_plate_swaps_the_x_and_y_results() {
    let r = readout(|_| {});
    let swapped = readout(|i| {
        i.set(InputField::Lx, "5.0");
        i.set(InputField::Ly, "4.0");
    });
    assert_eq!(swapped.center.mx, r.center.my);
    assert_eq!(swapped.edges[0].max_moment, r.edges[2].max_moment);
    assert_eq!(swapped.edges[2].max_reaction, r.edges[0].max_reaction);
}

#[test]
fn the_edge_moment_is_largest_at_the_middle_of_a_fixed_edge() {
    let r = readout(|_| {});
    let along: Vec<Option<&str>> = r
        .edges
        .iter()
        .map(|e| e.max_moment.along.as_deref())
        .collect();
    assert_eq!(along, ["2.50", "2.50", "2.00", "2.00"].map(Some));
}

#[test]
fn the_edge_moment_of_a_simply_supported_edge_is_zero_with_no_position() {
    let r = readout(|i| set_all(i, Support::SimplySupported));
    for row in &r.edges {
        assert_eq!(
            [
                row.max_moment.value.as_deref(),
                row.max_moment.along.as_deref()
            ],
            [Some("0"), None],
            "{}",
            row.name
        );
    }
}

/// 分布図の SVG の、等値線の上の数値（3 桁区切りの「,」を外して読む）
fn figure_numbers(svg: &str) -> Vec<f64> {
    svg.split("<text")
        .skip(1)
        .map(|t| {
            let text = &t[t.find('>').unwrap() + 1..t.find("</text>").unwrap()];
            text.replace(',', "")
                .parse()
                .unwrap_or_else(|_| panic!("数値でない: {text}"))
        })
        .collect()
}

#[test]
fn the_page_shows_the_deflection_figure_after_the_results_with_five_switches() {
    let main = element(&render_path("/tools/kirchhoff-plate"), "main").to_string();
    assert_in_order(
        &text_of(&main),
        &["釣り合い", "たわみ w（mm）", "w", "Mx", "My", "Qx", "Qy"],
    );
    // 既定はたわみ。押されている切替は 1 つ
    assert_eq!(main.matches("aria-pressed=\"true\"").count(), 1, "{main}");
    assert_eq!(main.matches("aria-pressed=\"false\"").count(), 4, "{main}");
    // ライトとダークの 2 枚。等値線の数値が読め、下向きの荷重でたわみは負にならない
    assert_eq!(main.matches("<svg xmlns").count(), 2, "{main}");
    let numbers = figure_numbers(&main);
    assert!(!numbers.is_empty(), "等値線の数値が無い: {main}");
    assert!(numbers.iter().all(|v| *v >= 0.0), "{numbers:?}");
    // 4 辺の支持条件を、図の名前と注記で言う
    assert!(
        main.contains(
            "aria-label=\"たわみ w の分布図。x = 0 の辺は固定、x = Lx の辺は固定、y = 0 の辺は固定、y = Ly の辺は固定\""
        ),
        "{main}"
    );
    assert_in_order(&text_of(&main), &["Qy", "固定", "単純支持", "自由"]);
}

#[test]
fn each_switch_names_its_figure_with_the_unit() {
    let names: Vec<(&str, &str)> = Quantity::ALL
        .iter()
        .map(|q| (q.symbol(), q.caption()))
        .collect();
    assert_eq!(
        names,
        [
            ("w", "たわみ w（mm）"),
            ("Mx", "曲げモーメント Mx（kN·m/m）"),
            ("My", "曲げモーメント My（kN·m/m）"),
            ("Qx", "せん断力 Qx（kN/m）"),
            ("Qy", "せん断力 Qy（kN/m）"),
        ]
    );
}

#[test]
fn a_moment_figure_of_the_fixed_plate_reads_both_signs() {
    // 四辺固定の Mx は中央で正、x = 0 と x = Lx の辺の近くで負
    let figure = PlateFigure::from_inputs(&PlateInputs::default(), Quantity::Mx).unwrap();
    let numbers = figure_numbers(&figure.light_svg);
    assert!(numbers.iter().any(|v| *v > 0.0), "{numbers:?}");
    assert!(numbers.iter().any(|v| *v < 0.0), "{numbers:?}");
    assert_eq!(figure_numbers(&figure.dark_svg), numbers);
}

#[test]
fn the_figure_follows_the_aspect_of_the_plate_within_one_column_and_names_each_support() {
    let size = |svg: &str| -> (f64, f64) {
        let attr = |name: &str| -> f64 {
            let at = svg.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
            svg[at..at + svg[at..].find('"').unwrap()].parse().unwrap()
        };
        (attr("width"), attr("height"))
    };
    let tall = PlateFigure::from_inputs(&PlateInputs::default(), Quantity::W).unwrap();
    let mut inputs = PlateInputs::default();
    inputs.set(InputField::Lx, "8.0");
    inputs.set(InputField::Ly, "2.0");
    inputs.set(InputField::Support(Edge::XLx), Support::Free.symbol());
    inputs.set(
        InputField::Support(Edge::Y0),
        Support::SimplySupported.symbol(),
    );
    let wide = PlateFigure::from_inputs(&inputs, Quantity::W).unwrap();
    let ((tw, th), (ww, wh)) = (size(&tall.light_svg), size(&wide.light_svg));
    // 長い辺の向きが 1 単位（335）いっぱい
    assert_eq!((th, ww), (335.0, 335.0), "{tw}×{th} {ww}×{wh}");
    // 4:5 の板の大きさから縁の幅 m を読む（tw − 2m = 0.8 (th − 2m)）。8:2 の板も、その縁を除いて 4:1
    let m = (tw - 0.8 * th) / 0.4;
    assert!((0.0..20.0).contains(&m), "縁 {m}");
    assert!(
        ((ww - 2.0 * m) / (wh - 2.0 * m) - 4.0).abs() < 1e-3,
        "{tw}×{th} {ww}×{wh}"
    );
    assert_eq!(
        wide.label,
        "たわみ w の分布図。x = 0 の辺は固定、x = Lx の辺は自由、y = 0 の辺は単純支持、y = Ly の辺は固定"
    );
}

#[test]
fn a_plate_that_cannot_be_solved_has_no_figure() {
    let mut unstable = PlateInputs::default();
    set_all(&mut unstable, Support::Free);
    assert_eq!(PlateFigure::from_inputs(&unstable, Quantity::W), None);
    let mut unreadable = PlateInputs::default();
    unreadable.set(InputField::Lx, "0");
    assert_eq!(PlateFigure::from_inputs(&unreadable, Quantity::W), None);
}

/// 三辺固定一辺自由（y = Ly の辺が自由）の入力。長い辺（x、5 m）が自由
fn three_fixed_one_free() -> PlateInputs {
    let mut inputs = PlateInputs::default();
    inputs.set(InputField::Lx, "5.0");
    inputs.set(InputField::Ly, "4.0");
    inputs.set(InputField::Support(Edge::YLy), Support::Free.symbol());
    inputs
}

#[test]
fn the_deflection_maximum_of_the_fixed_plate_is_the_centre() {
    // 求め方（要素の中まで見る、ADR kirchhoff-plate-0009）は計算核のテスト。ここでは表の値と位置の書き方
    let r = readout(|_| {});
    let d = &r.deflection_maximum;
    assert_eq!(d.w, r.center.w);
    assert_eq!(
        [d.x.as_deref(), d.y.as_deref()],
        [Some("2.00"), Some("2.50")]
    );
}

#[test]
fn with_a_free_edge_the_deflection_maximum_is_on_that_edge() {
    let r = PlateReadout::from_inputs(&three_fixed_one_free());
    let d = &r.deflection_maximum;
    assert_eq!(
        [d.x.as_deref(), d.y.as_deref()],
        [Some("2.50"), Some("4.00")]
    );
    let w = |w: &Option<String>| w.as_deref().expect("解ける").parse::<f64>().unwrap();
    assert!(w(&d.w) > w(&r.center.w), "{d:?} {:?}", r.center);
    // 自由辺を y = 0 に移すと位置は 0
    let mut inputs = three_fixed_one_free();
    inputs.set(InputField::Support(Edge::YLy), Support::Fixed.symbol());
    inputs.set(InputField::Support(Edge::Y0), Support::Free.symbol());
    let moved = PlateReadout::from_inputs(&inputs).deflection_maximum;
    assert_eq!(
        [moved.x.as_deref(), moved.y.as_deref()],
        [Some("2.50"), Some("0")]
    );
    assert_eq!(moved.w, d.w);
}

#[test]
fn a_plate_with_a_free_edge_says_how_the_edge_shear_and_reaction_are_taken() {
    // 両端から辺長の 5 % を除いて取る規則（ADR kirchhoff-plate-0008）は計算核の `edge_maximum` のテスト。
    // ここでは注記の文言
    let r = PlateReadout::from_inputs(&three_fixed_one_free());
    assert_eq!(
        r.edge_notes,
        [
            "固定辺と自由辺の隅では、せん断力と辺の反力が発散する",
            "辺のせん断力と反力の最大値は、隅から辺長の 5 % を除いた範囲で取る",
        ]
    );
}

#[test]
fn a_plate_without_a_free_edge_has_no_notes_on_the_corners() {
    for supports in [Support::Fixed, Support::SimplySupported] {
        let r = readout(|i| set_all(i, supports));
        assert!(r.edge_notes.is_empty(), "{:?}", r.edge_notes);
    }
    let text = text_of(element(&render_path("/tools/kirchhoff-plate"), "main"));
    assert!(!text.contains("発散"), "{text}");
    assert!(!text.contains("5 %"), "{text}");
}

#[test]
fn the_shear_figure_of_a_plate_with_a_free_edge_is_read_inside_the_plate() {
    // 隅の発散で色の幅を決めると、刻みが粗くなって板の中に等値線がほとんど引かれない。色の幅は隅から
    // 辺長の 5 % 以内の点を除いて決める（ADR kirchhoff-plate-0008）ので、板の中の Qx に正の水準が 3 つ以上読める
    let figure = PlateFigure::from_inputs(&three_fixed_one_free(), Quantity::Qx).unwrap();
    let mut positive: Vec<f64> = figure_numbers(&figure.light_svg)
        .into_iter()
        .filter(|v| *v > 0.0)
        .collect();
    positive.sort_by(f64::total_cmp);
    positive.dedup();
    assert!(positive.len() >= 3, "{positive:?}");
    assert_eq!(
        figure.notes,
        ["色の幅は、隅から辺長の 5 % 以内の点を除いて決める"]
    );
    let fixed = PlateFigure::from_inputs(&PlateInputs::default(), Quantity::Qx).unwrap();
    assert!(fixed.notes.is_empty(), "{:?}", fixed.notes);
}

#[test]
fn a_negative_poisson_ratio_is_named_with_the_range_of_poisson_ratio() {
    let r = readout(|i| i.set(InputField::PoissonRatio, "-0.1"));
    assert_eq!(
        reasons(&r.errors),
        [(InputField::PoissonRatio, "0 以上 0.5 未満で入れる")]
    );
    assert_not_computed(&r);
}
