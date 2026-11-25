//! Kirchhoff 板の曲げ（Issue #36）: `/tools` の一覧と `/tools/kirchhoff-plate` のページ。
//! 言葉は `docs/tools/kirchhoff-plate/GLOSSARY.md`。数値は `rustruct-calc` が出し、ここでは表示の文字列を見る。

mod common;
use charta::{Item, Paint, Palette, Scene};
use common::markup::count_tags;
use common::{
    Said, assert_in_order, element, flagged_heads, input_groups, paragraphs, said, text_of,
};
use numerus::significant;
use rustruct_calc::kirchhoff_plate::{PointResponse, Solution, solve};
use rustruct_types::kirchhoff_plate::{Edge, Plate, PlateMaterial, Support, Supports};
use rustruct_types::quantity::{AreaLoad, Length, PoissonRatio, Stress};
use rustruct_web::{
    InputField, Material, PlateFigure, PlateInputs, PlateOutcome, PlateReadout, PointTerm, Pointed,
    Pointer, Quantity, material, render_path, render_path_with, switch_material,
};

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
    // ν は既定が 0 なので、入力の表の下で案内しない（ADR kirchhoff-plate-0005）
    assert!(!text_of(main).contains("基本"), "ν の注記がある: {main}");
    for value in ["4.0", "5.0", "150", "22700", "0", "10"] {
        assert!(
            main.contains(&format!("value=\"{value}\"")),
            "入力欄に初期値 {value} が無い: {main}"
        );
    }
    // 四辺とも、選択の欄（リストを開くボタン）に固定が選ばれている
    assert_eq!(
        main.matches("aria-haspopup=\"listbox\"").count(),
        4,
        "{main}"
    );
    assert_in_order(
        &text_of(main),
        &[
            "x = 0（左）",
            "固定",
            "x = Lx（右）",
            "固定",
            "y = 0（下）",
            "固定",
            "y = Ly（上）",
            "固定",
            "板の中央",
        ],
    );
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
    // 「直接入力」は材料の切り替えの名前
    assert!(
        !text.replace("直接入力", "").contains("入力"),
        "部の見出し「入力」がある: {text}"
    );
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

#[test]
fn the_edge_notes_do_not_say_that_m_is_zero_on_simply_supported_and_free_edges() {
    // 支持条件から明らかなので言わない
    let text = text_of(element(&render_path("/tools/kirchhoff-plate"), "main"));
    assert!(text.contains("辺上の絶対値の最大値"), "{text}");
    assert!(!text.contains("M は 0"), "{text}");
}

fn inputs(change: impl FnOnce(&mut PlateInputs)) -> PlateInputs {
    let mut inputs = PlateInputs::default();
    change(&mut inputs);
    inputs
}

fn outcome(change: impl FnOnce(&mut PlateInputs)) -> PlateOutcome {
    PlateOutcome::from_inputs(&inputs(change))
}

fn readout(change: impl FnOnce(&mut PlateInputs)) -> PlateReadout {
    outcome(change).readout
}

/// 入力を打った後のページの、入力の組の HTML（組は 1 つ）
fn group(change: impl FnOnce(&mut PlateInputs)) -> String {
    let html = render_path_with("/tools/kirchhoff-plate", inputs(change));
    let groups = input_groups(element(&html, "main"));
    let [group] = groups[..] else {
        panic!("入力の組が 1 つでない: {html}");
    };
    group.to_string()
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
    let o = outcome(|i| set_all(i, Support::Free));
    assert_eq!(
        said(&o.record),
        Said {
            warnings: vec!["この支持条件の組では板を支えられない"],
            ..Said::default()
        }
    );
    assert_not_computed(&o.readout);
}

#[test]
fn a_combination_with_a_free_edge_is_solved() {
    // 不安定な組以外はすべて解ける（ADR kirchhoff-plate-0002）。釣り合いと自由辺の反力は計算核のテスト
    let o = outcome(|i| i.set(InputField::Support(Edge::XLx), Support::Free.symbol()));
    assert_eq!(said(&o.record), Said::default());
    let r = o.readout;
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
    let o = outcome(|i| set_all(i, Support::SimplySupported));
    assert_eq!(said(&o.record), Said::default());
    let r = o.readout;
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
    let o = outcome(|i| {
        i.set(InputField::Lx, "0");
        i.set(InputField::Thickness, "-150");
        i.set(InputField::YoungModulus, "abc");
        i.set(InputField::PoissonRatio, "0.5");
        i.set(InputField::Load, "");
    });
    // 読めない欄があるあいだは、解けない理由（警告）を言わない
    assert_eq!(
        said(&o.record),
        Said {
            reasons: vec![
                (InputField::Lx, "0 より大きい値を入れる"),
                (InputField::Thickness, "0 より大きい値を入れる"),
                (InputField::YoungModulus, "数値で入れる"),
                (InputField::PoissonRatio, "0 以上 0.5 未満で入れる"),
                (InputField::Load, "値を入れる"),
            ],
            ..Said::default()
        }
    );
    assert_not_computed(&o.readout);
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

/// 入力から、ページと同じ経路で 1 つの量の分布図。解けなければ図は無い
fn figure_of(inputs: &PlateInputs, quantity: Quantity) -> Option<PlateFigure> {
    let field = PlateOutcome::from_inputs(inputs).field?;
    Some(PlateFigure::new(&field, quantity))
}

/// 分布図の文字。どれも等値線の上の数値（3 桁区切りの「,」を外して読む）
fn figure_numbers(scene: &Scene) -> Vec<f64> {
    scene
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, .. } => Some(
                text.replace(',', "")
                    .parse()
                    .unwrap_or_else(|_| panic!("数値でない: {text}")),
            ),
            _ => None,
        })
        .collect()
}

/// 帯の色の列
fn band_palettes(scene: &Scene) -> Vec<Palette> {
    scene
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Path {
                closed: true,
                fill: Some(Paint::Band { palette, .. }),
                ..
            } => Some(*palette),
            _ => None,
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
    let quantities = switch_group(&main, "分布図の量");
    let pressed = |value| count_tags(quantities, "button", &[("aria-pressed", value)]);
    assert_eq!((pressed("true"), pressed("false")), (1, 4), "{main}");
    // 切替は名前の付いた組
    assert_eq!(
        count_tags(
            &main,
            "div",
            &[("aria-label", "分布図の量"), ("role", "group")]
        ),
        1,
        "{main}"
    );
    // 図は 1 枚（ライトとダークは CSS 変数で切り替わる）
    assert_eq!(main.matches("<svg xmlns").count(), 1, "{main}");
    // 4 辺の支持条件は図の名前で言う。図の下で縁の読み方は言い直さない（入力の表が支持条件を言う）
    assert!(
        main.contains(
            "aria-label=\"たわみ w の分布図。x = 0 の辺は固定、x = Lx の辺は固定、y = 0 の辺は固定、y = Ly の辺は固定\""
        ),
        "{main}"
    );
    let after_switches = &text_of(&main)[text_of(&main).rfind("Qy").unwrap()..];
    for support in ["固定", "単純支持", "自由"] {
        assert!(!after_switches.contains(support), "{after_switches}");
    }
}

/// ページの、分布図より後の HTML（図の列は最後）
fn after_the_figure(inputs: PlateInputs) -> String {
    let main = element(&render_path_with("/tools/kirchhoff-plate", inputs), "main").to_string();
    let end = main.rfind("</svg>").expect("図がある") + "</svg>".len();
    main[end..].to_string()
}

#[test]
fn the_figure_has_notes_only_when_the_figure_gives_some() {
    // 自由辺の無い組の図には注記が無く、空の箇条も置かない
    let fixed = after_the_figure(PlateInputs::default());
    assert!(!fixed.contains("<ul"), "{fixed}");
    let free = after_the_figure(three_fixed_one_free());
    assert_eq!(free.matches("<li").count(), 1, "{free}");
    assert!(
        text_of(&free).contains("色の幅は、隅から辺長の 5 % 以内の点を除いて決める"),
        "{free}"
    );
}

#[test]
fn each_switch_names_its_figure_with_the_unit() {
    let names: Vec<(&str, String)> = Quantity::ALL
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
        .map(|(symbol, caption)| (symbol, caption.to_string()))
    );
}

#[test]
fn a_moment_figure_of_the_fixed_plate_reads_both_signs() {
    // 四辺固定の Mx は中央で正、x = 0 と x = Lx の辺の近くで負
    let figure = figure_of(&PlateInputs::default(), Quantity::Mx).unwrap();
    let numbers = figure_numbers(&figure.scene);
    assert!(numbers.iter().any(|v| *v > 0.0), "{numbers:?}");
    assert!(numbers.iter().any(|v| *v < 0.0), "{numbers:?}");
}

#[test]
fn the_deflection_reads_no_negative_values_and_each_quantity_bands_in_the_palette_of_its_signs() {
    // 下向きの荷重でたわみは負にならず、片符号の量は順の色、両符号の量は発散の色で塗る
    let w = figure_of(&PlateInputs::default(), Quantity::W).unwrap();
    let numbers = figure_numbers(&w.scene);
    assert!(!numbers.is_empty(), "等値線の数値が無い: {:?}", w.scene);
    assert!(numbers.iter().all(|v| *v >= 0.0), "{numbers:?}");
    for (quantity, palette) in [(Quantity::W, Palette::Seq), (Quantity::Mx, Palette::Div)] {
        let scene = figure_of(&PlateInputs::default(), quantity).unwrap().scene;
        let palettes = band_palettes(&scene);
        assert!(!palettes.is_empty(), "{quantity:?}");
        assert!(palettes.iter().all(|p| *p == palette), "{palettes:?}");
    }
}

#[test]
fn the_figure_follows_the_aspect_of_the_plate_within_one_column_and_names_each_support() {
    let size = |scene: &Scene| (scene.width, scene.height);
    let tall = figure_of(&PlateInputs::default(), Quantity::W).unwrap();
    let mut inputs = PlateInputs::default();
    inputs.set(InputField::Lx, "8.0");
    inputs.set(InputField::Ly, "2.0");
    inputs.set(InputField::Support(Edge::XLx), Support::Free.symbol());
    inputs.set(
        InputField::Support(Edge::Y0),
        Support::SimplySupported.symbol(),
    );
    let wide = figure_of(&inputs, Quantity::W).unwrap();
    let ((tw, th), (ww, wh)) = (size(&tall.scene), size(&wide.scene));
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
fn the_figure_holds_the_plate_as_its_plot_inside_the_margin_and_returns_points_of_the_plate() {
    let mut inputs = PlateInputs::default();
    inputs.set(InputField::Lx, "8.0");
    inputs.set(InputField::Ly, "2.0");
    let scene = figure_of(&inputs, Quantity::W).unwrap().scene;
    assert_eq!(scene.plots.len(), 1, "{:?}", scene.plots);
    // 描く範囲は、四辺に同じ幅の縁を残して内側にずれている
    let [lo, hi] = scene.plots[0].bounds();
    let m = lo[0];
    assert!(m > 0.0, "縁 {m}");
    assert_eq!(
        [lo[1], scene.width - hi[0], scene.height - hi[1]],
        [m, m, m],
        "{lo:?} {hi:?}"
    );
    // 描く範囲の 4 隅と中心は、板の 4 隅と中心（y は上が正）
    let center = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    for (screen, data) in [
        ([lo[0], hi[1]], [0.0, 0.0]),
        ([hi[0], hi[1]], [8.0, 0.0]),
        ([lo[0], lo[1]], [0.0, 2.0]),
        ([hi[0], lo[1]], [8.0, 2.0]),
        (center, [4.0, 1.0]),
    ] {
        let (k, p) = scene.locate(screen).expect("板の中");
        assert_eq!(k, 0);
        assert!(
            (p[0] - data[0]).abs() < 1e-12 && (p[1] - data[1]).abs() < 1e-12,
            "{screen:?} → {p:?}"
        );
    }
    // 縁（支持条件の斜線の所）は当たらない
    for margin in [
        [m / 2.0, center[1]],
        [scene.width - m / 2.0, center[1]],
        [center[0], m / 2.0],
        [center[0], scene.height - m / 2.0],
    ] {
        assert_eq!(scene.locate(margin), None, "{margin:?}");
    }
}

/// 初期値の板（4 × 5 m、四辺固定）を計算核で直接解いたもの
fn initial_solution() -> Solution {
    let fixed = Support::Fixed;
    solve(&Plate {
        lx: Length::m(4.0),
        ly: Length::m(5.0),
        thickness: Length::mm(150.0),
        material: PlateMaterial::Direct {
            young_modulus: Stress::n_per_mm2(22700.0),
            poisson_ratio: PoissonRatio::new(0.0).unwrap(),
        },
        load: AreaLoad::kn_per_m2(10.0),
        supports: Supports {
            x0: fixed,
            y0: fixed,
            x_lx: fixed,
            y_ly: fixed,
        },
    })
    .unwrap()
}

/// 図の量の、計算核の点の応答での値（図の名前の単位）
fn value_of(quantity: Quantity, r: &PointResponse) -> f64 {
    match quantity {
        Quantity::W => r.w.as_mm(),
        Quantity::Mx => r.mx.as_knm_per_m(),
        Quantity::My => r.my.as_knm_per_m(),
        Quantity::Qx => r.qx.as_kn_per_m(),
        Quantity::Qy => r.qy.as_kn_per_m(),
    }
}

/// 点の値の項を「x = 1.00 m」の形の文字に（値が無ければ「—」）
fn terms_text(terms: &[PointTerm]) -> Vec<String> {
    terms
        .iter()
        .map(|t| {
            let value = t.value.clone().unwrap_or_else(|| "—".into());
            format!("{} = {value} {}", t.symbol, t.unit)
        })
        .collect()
}

/// 図の画面の点 `screen` に止めた、指した点
fn pinned_at(scene: &Scene, screen: [f64; 2]) -> Pointer {
    let (plot, at) = scene.locate(screen).expect("板の中");
    Pointer::Pinned(Pointed { plot, at })
}

#[test]
fn the_point_value_is_the_value_of_the_solution_at_that_point_for_each_quantity() {
    let field = PlateOutcome::from_inputs(&PlateInputs::default())
        .field
        .unwrap();
    let solution = initial_solution();
    for at in [[1.0, 1.5], [0.37, 4.12], [2.0, 2.5], [4.0, 0.0]] {
        let r = solution.at(Length::m(at[0]), Length::m(at[1])).unwrap();
        for quantity in Quantity::ALL {
            let unit = match quantity {
                Quantity::W => "mm",
                Quantity::Mx | Quantity::My => "kN·m/m",
                Quantity::Qx | Quantity::Qy => "kN/m",
            };
            assert_eq!(
                terms_text(&field.point_value(quantity, at)),
                [
                    format!("x = {} m", significant(at[0], 3)),
                    format!("y = {} m", significant(at[1], 3)),
                    format!(
                        "{} = {} {unit}",
                        quantity.symbol(),
                        significant(value_of(quantity, &r), 3)
                    ),
                ],
                "{at:?} {quantity:?}"
            );
        }
    }
    // 中央の点の値は、結果の表の板の中央の値と同じ文字
    let center = PlateOutcome::from_inputs(&PlateInputs::default())
        .readout
        .center;
    for (quantity, table) in [
        (Quantity::W, center.w),
        (Quantity::Mx, center.mx),
        (Quantity::My, center.my),
    ] {
        assert_eq!(field.point_value(quantity, [2.0, 2.5])[2].value, table);
    }
}

#[test]
fn a_pinned_point_is_kept_across_the_quantities_and_the_lengths_and_cleared_off_the_plate() {
    let solution = initial_solution();
    let w = figure_of(&PlateInputs::default(), Quantity::W)
        .unwrap()
        .scene;
    let [lo, hi] = w.plots[0].bounds();
    // 描く範囲の左から 3/4、下から 1/5 の所（x = 3 m、y = 1 m）
    let pinned = pinned_at(
        &w,
        [
            lo[0] + 0.75 * (hi[0] - lo[0]),
            hi[1] - 0.2 * (hi[1] - lo[1]),
        ],
    );
    let at = pinned.point().unwrap().at;
    assert!(
        (at[0] - 3.0).abs() < 1e-9 && (at[1] - 1.0).abs() < 1e-9,
        "{at:?}"
    );

    // 量を切り替えても同じ点で、その量の値
    let field = PlateOutcome::from_inputs(&PlateInputs::default())
        .field
        .unwrap();
    let mx = figure_of(&PlateInputs::default(), Quantity::Mx)
        .unwrap()
        .scene;
    assert_eq!(pinned.within(&mx), pinned);
    let r = solution.at(Length::m(at[0]), Length::m(at[1])).unwrap();
    assert_eq!(
        field.point_value(Quantity::Mx, at)[2].value,
        Some(significant(r.mx.as_knm_per_m(), 3))
    );

    // 辺長を変えると、同じ点で新しい解の値を出し直す
    let longer = inputs(|i| i.set(InputField::Lx, "6.0"));
    let scene = figure_of(&longer, Quantity::W).unwrap().scene;
    assert_eq!(pinned.within(&scene), pinned);
    let moved = PlateOutcome::from_inputs(&longer)
        .field
        .unwrap()
        .point_value(Quantity::W, at);
    assert_ne!(moved, field.point_value(Quantity::W, at));
    assert_eq!(terms_text(&moved[..2]), ["x = 3.00 m", "y = 1.00 m"]);

    // 点が板の外に出る辺長では消える
    let shorter = figure_of(&inputs(|i| i.set(InputField::Lx, "2.5")), Quantity::W)
        .unwrap()
        .scene;
    assert_eq!(pinned.within(&shorter), Pointer::Nothing);
}

#[test]
fn the_page_says_how_to_point_between_the_figure_and_its_notes() {
    // 図を描く前（SSG）は読み方の文。十字線の層は無く、図は読み上げの名前の付いた img
    let free = element(
        &render_path_with("/tools/kirchhoff-plate", three_fixed_one_free()),
        "main",
    )
    .to_string();
    assert_in_order(
        &text_of(&free),
        &[
            "Qy",
            HINT,
            "色の幅は、隅から辺長の 5 % 以内の点を除いて決める",
        ],
    );
    assert_eq!(count_tags(&free, "figure", &[("role", "img")]), 1, "{free}");
    assert_eq!(free.matches("<svg xmlns").count(), 1, "{free}");
    // 解けないときは図も読み方の文も無い
    let mut unstable = PlateInputs::default();
    set_all(&mut unstable, Support::Free);
    let main = element(
        &render_path_with("/tools/kirchhoff-plate", unstable),
        "main",
    )
    .to_string();
    assert!(!main.contains(HINT), "{main}");
}

#[test]
fn the_figure_of_the_initial_inputs_is_at_most_96_497_bytes_of_svg() {
    // 帯を範囲ごとの外周で描く（#128）。上限は、帯をセルと三角形の小片で敷き詰めていたときの 385,989 B の 1/4
    const LIMIT: usize = 96_497;
    let main = element(&render_path("/tools/kirchhoff-plate"), "main").to_string();
    let start = main.find("<svg xmlns").expect("図がある");
    let end = start + main[start..].find("</svg>").expect("図が閉じる") + "</svg>".len();
    let size = end - start;
    assert!(size <= LIMIT, "図の SVG {size} B");
}

#[test]
fn a_plate_that_cannot_be_solved_has_no_figure() {
    let mut unstable = PlateInputs::default();
    set_all(&mut unstable, Support::Free);
    assert_eq!(figure_of(&unstable, Quantity::W), None);
    let mut unreadable = PlateInputs::default();
    unreadable.set(InputField::Lx, "0");
    assert_eq!(figure_of(&unreadable, Quantity::W), None);
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
    let r = PlateOutcome::from_inputs(&three_fixed_one_free()).readout;
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
    let moved = PlateOutcome::from_inputs(&inputs)
        .readout
        .deflection_maximum;
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
    let r = PlateOutcome::from_inputs(&three_fixed_one_free()).readout;
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
    let figure = figure_of(&three_fixed_one_free(), Quantity::Qx).unwrap();
    let mut positive: Vec<f64> = figure_numbers(&figure.scene)
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
    let fixed = figure_of(&PlateInputs::default(), Quantity::Qx).unwrap();
    assert!(fixed.notes.is_empty(), "{:?}", fixed.notes);
}

#[test]
fn a_negative_poisson_ratio_is_named_with_the_range_of_poisson_ratio() {
    let o = outcome(|i| i.set(InputField::PoissonRatio, "-0.1"));
    assert_eq!(
        said(&o.record),
        Said {
            reasons: vec![(InputField::PoissonRatio, "0 以上 0.5 未満で入れる")],
            ..Said::default()
        }
    );
    assert_not_computed(&o.readout);
}

// 入力の組の HTML（`scripts/screenshot.sh` は欄に打てないので、打った後の状態はここで見る）

/// 分布図の点の値の行の、何も指していないときの読み方の文。組の下の段落で、図があればいつもある
const HINT: &str = "図を指すとその点の値を出す";

#[test]
fn the_initial_group_says_nothing() {
    // 組の下の段落は、分布図の点の値の行の読み方の文だけ
    let group = group(|_| {});
    assert_eq!(paragraphs(&group), [HINT], "{group}");
    assert_eq!(flagged_heads(&group), 0, "{group}");
}

#[test]
fn unreadable_fields_and_a_poisson_ratio_out_of_range_are_said_under_the_sheet() {
    let group = group(|i| {
        i.set(InputField::Lx, "x");
        i.set(InputField::PoissonRatio, "0.5");
    });
    // 分布図は解けないので「—」
    assert_eq!(
        paragraphs(&group),
        ["Lx（m）: 数値で入れる", "ν: 0 以上 0.5 未満で入れる", "—"]
    );
    assert_eq!(flagged_heads(&group), 2, "{group}");
}

#[test]
fn an_unstable_combination_is_a_warning_without_flags() {
    let group = group(|i| set_all(i, Support::Free));
    assert_eq!(
        paragraphs(&group),
        ["! この支持条件の組では板を支えられない", "—"]
    );
    assert_eq!(flagged_heads(&group), 0, "{group}");
}

/// 名前の付いた切替の組（`role="group"` の `div`）
fn switch_group<'a>(html: &'a str, label: &str) -> &'a str {
    let at = html
        .find(&format!("aria-label=\"{label}\""))
        .unwrap_or_else(|| panic!("切替の組「{label}」が無い: {html}"));
    let start = html[..at].rfind('<').expect("開きタグ");
    let group = element(&html[start..], "div");
    let tag = &common::markup::tags(group)[0];
    assert_eq!(tag.get("role"), Some("group"), "{group}");
    group
}

/// 切替の組の、押されているボタンの文字
fn pressed_names(group: &str) -> Vec<String> {
    common::markup::elements(group, "button")
        .into_iter()
        .filter(|button| {
            common::markup::tags(button)
                .first()
                .is_some_and(|tag| tag.get("aria-pressed") == Some("true"))
        })
        .map(text_of)
        .collect()
}

// 板の材料（#135）

#[test]
fn the_page_opens_on_direct_input_with_the_material_switch_above_the_sheet() {
    let main = element(&render_path("/tools/kirchhoff-plate"), "main").to_string();
    let materials = switch_group(&main, "板の材料");
    assert_in_order(&text_of(materials), &["コンクリート", "鋼材", "直接入力"]);
    assert_eq!(pressed_names(materials), ["直接入力"], "{materials}");
    assert_eq!(
        count_tags(materials, "button", &[("aria-pressed", "false")]),
        2
    );
    // 切り替えは入力の組のいちばん上（Sheet の前）
    assert_in_order(&text_of(&main), &["直接入力", "Lx（m）", "板の中央"]);
    // 直接入力では E と ν は書ける欄で、読むだけの欄も Fc も無い
    assert_eq!(main.matches("<output").count(), 0, "{main}");
    assert!(!main.contains("Fc"), "{main}");
}

#[test]
fn switching_to_direct_input_from_concrete_takes_the_rounded_e_and_the_nu_of_concrete() {
    let mut inputs = PlateInputs::default();
    switch_material(&mut inputs, Material::Concrete);
    assert_eq!(material(&inputs), Material::Concrete);
    assert_eq!(inputs.value(InputField::Fc), "24");
    assert_eq!(inputs.value(InputField::ConcretePoissonRatio), "0.2");
    switch_material(&mut inputs, Material::Direct);
    assert_eq!(material(&inputs), Material::Direct);
    assert_eq!(inputs.value(InputField::YoungModulus), "22669");
    assert_eq!(inputs.value(InputField::PoissonRatio), "0.2");
}

#[test]
fn switching_back_to_concrete_restores_the_fc_and_nu_entered_for_concrete() {
    let mut inputs = PlateInputs::default();
    switch_material(&mut inputs, Material::Concrete);
    inputs.set(InputField::Fc, "30");
    inputs.set(InputField::ConcretePoissonRatio, "0.15");
    switch_material(&mut inputs, Material::Direct);
    assert_eq!(inputs.value(InputField::PoissonRatio), "0.15");
    // 直接入力で書き換えても、コンクリートの値は残る
    inputs.set(InputField::PoissonRatio, "0.1");
    switch_material(&mut inputs, Material::Concrete);
    assert_eq!(inputs.value(InputField::Fc), "30");
    assert_eq!(inputs.value(InputField::ConcretePoissonRatio), "0.15");
}

#[test]
fn switching_to_direct_input_from_steel_takes_205000_and_0_3() {
    let mut inputs = PlateInputs::default();
    switch_material(&mut inputs, Material::Steel);
    switch_material(&mut inputs, Material::Direct);
    assert_eq!(inputs.value(InputField::YoungModulus), "205000");
    assert_eq!(inputs.value(InputField::PoissonRatio), "0.3");
}

/// 直前の材料の値が使えないときは、その欄は前のまま
#[test]
fn an_unusable_value_of_concrete_leaves_its_direct_field_as_it_was() {
    let mut inputs = PlateInputs::default();
    switch_material(&mut inputs, Material::Concrete);
    inputs.set(InputField::Fc, "x");
    inputs.set(InputField::ConcretePoissonRatio, "0.5");
    switch_material(&mut inputs, Material::Direct);
    assert_eq!(inputs.value(InputField::YoungModulus), "22700");
    assert_eq!(inputs.value(InputField::PoissonRatio), "0");
}

#[test]
fn concrete_shows_fc_and_nu_to_write_and_e_to_read_as_an_integer() {
    let group = group(|i| switch_material(i, Material::Concrete));
    assert_eq!(
        pressed_names(&render_switch(Material::Concrete)),
        ["コンクリート"]
    );
    // 並びは t・E、ν・p の行を保ち、Fc を下の行に足す
    assert_in_order(
        &text_of(&group),
        &[
            "Lx（m）",
            "Ly（m）",
            "t（mm）",
            "E（N/mm²）",
            "22669",
            "ν",
            "p（kN/m²）",
            "Fc（N/mm²）",
            "x = 0（左）",
        ],
    );
    for (label, value) in [("Fc（N/mm²）", "24"), ("ν", "0.2")] {
        assert_eq!(
            count_tags(&group, "input", &[("aria-label", label), ("value", value)]),
            1,
            "{group}"
        );
    }
    assert_eq!(
        count_tags(&group, "output", &[("aria-label", "E（N/mm²）")]),
        1,
        "{group}"
    );
    assert_eq!(
        count_tags(&group, "input", &[("aria-label", "E（N/mm²）")]),
        0
    );
    // Fc の行の空いた組は、2 列にまたがる何も無いセル（書く欄も読む欄も文字も無い）
    let fc_row = common::markup::elements(&group, "tr")
        .into_iter()
        .find(|row| row.contains("Fc（N/mm²）"))
        .unwrap_or_else(|| panic!("Fc の行が無い: {group}"));
    let blanks: Vec<&str> = common::markup::elements(fc_row, "td")
        .into_iter()
        .filter(|cell| count_tags(cell, "td", &[("colspan", "2")]) == 1)
        .collect();
    assert_eq!(blanks.len(), 1, "{fc_row}");
    assert_eq!(text_of(blanks[0]), "", "{fc_row}");
    assert_eq!(
        count_tags(blanks[0], "input", &[]) + count_tags(blanks[0], "output", &[]),
        0,
        "{fc_row}"
    );
    assert_eq!(paragraphs(&group), [HINT], "{group}");
}

#[test]
fn steel_shows_e_and_nu_to_read() {
    let group = group(|i| switch_material(i, Material::Steel));
    assert_in_order(
        &text_of(&group),
        &["t（mm）", "E（N/mm²）", "205000", "ν", "0.3", "p（kN/m²）"],
    );
    assert_eq!(count_tags(&group, "output", &[]), 2, "{group}");
    assert_eq!(
        count_tags(&group, "input", &[("aria-label", "ν")]),
        0,
        "{group}"
    );
    assert!(!group.contains("Fc"), "{group}");
    assert_eq!(paragraphs(&group), [HINT], "{group}");
}

#[test]
fn an_fc_below_18_is_a_warning_of_the_input_group_and_the_plate_is_still_solved() {
    let change = |i: &mut PlateInputs| {
        switch_material(i, Material::Concrete);
        i.set(InputField::Fc, "15");
    };
    let group = group(change);
    assert_eq!(
        paragraphs(&group),
        ["! Fc が下限 18 N/mm² に満たない", HINT],
        "{group}"
    );
    assert_eq!(flagged_heads(&group), 0, "{group}");
    assert!(outcome(change).readout.center.w.is_some());
}

#[test]
fn an_unreadable_fc_is_named_and_e_is_not_shown() {
    let group = group(|i| {
        switch_material(i, Material::Concrete);
        i.set(InputField::Fc, "");
    });
    assert_eq!(
        paragraphs(&group),
        ["Fc（N/mm²）: 値を入れる", "—"],
        "{group}"
    );
    assert_eq!(flagged_heads(&group), 1, "{group}");
    let output = common::markup::elements(&group, "output");
    assert_eq!(output.iter().map(|o| text_of(o)).collect::<Vec<_>>(), ["—"]);
}

/// 材料を切り替えた後のページの、材料の切替の組
fn render_switch(to: Material) -> String {
    let html = render_path_with("/tools/kirchhoff-plate", inputs(|i| switch_material(i, to)));
    switch_group(element(&html, "main"), "板の材料").to_string()
}

#[test]
fn the_switch_says_the_chosen_material_with_aria_pressed() {
    for to in Material::ALL {
        assert_eq!(pressed_names(&render_switch(to)), [to.name()]);
    }
}
