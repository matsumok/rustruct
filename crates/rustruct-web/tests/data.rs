//! Data（Issue #19）: `/data` とその下のコンクリート・鉄筋のページ。
//! 数値は `rustruct-types` が出し、ここでは表示の文字列を見る。期待値は `docs/materials/adr/` の表の写し。

mod common;
use common::{assert_in_order, text_of};
use rustruct_web::{ConcreteReadout, render_path};

#[test]
fn data_lists_concrete_then_rebar_with_path_name_and_blurb() {
    let html = render_path("/data");
    assert_in_order(
        &text_of(&html),
        &[
            "/data",
            "Data",
            "/data/concrete",
            "コンクリート",
            "/data/rebar",
            "鉄筋",
        ],
    );
    for path in ["/data/concrete", "/data/rebar"] {
        assert!(
            html.contains(&format!("href=\"{path}\"")),
            "{path} へのリンクが無い: {html}"
        );
    }
}

/// 呼び名, d, l, S, w。JIS G 3112 表6（ADR-0002 の写し）と同じ桁の文字列
const REBAR_ROWS: [[&str; 5]; 16] = [
    ["D4", "4.23", "13.3", "14.05", "0.110"],
    ["D5", "5.29", "16.6", "21.98", "0.173"],
    ["D6", "6.35", "20.0", "31.67", "0.249"],
    ["D8", "7.94", "24.9", "49.51", "0.389"],
    ["D10", "9.53", "29.9", "71.33", "0.560"],
    ["D13", "12.7", "39.9", "126.7", "0.995"],
    ["D16", "15.9", "50.0", "198.6", "1.56"],
    ["D19", "19.1", "60.0", "286.5", "2.25"],
    ["D22", "22.2", "69.8", "387.1", "3.04"],
    ["D25", "25.4", "79.8", "506.7", "3.98"],
    ["D29", "28.6", "89.9", "642.4", "5.04"],
    ["D32", "31.8", "99.9", "794.2", "6.23"],
    ["D35", "34.9", "109.7", "956.6", "7.51"],
    ["D38", "38.1", "119.7", "1140", "8.95"],
    ["D41", "41.3", "129.8", "1340", "10.5"],
    ["D51", "50.8", "159.6", "2027", "15.9"],
];

#[test]
fn rebar_table_shows_every_designation_with_the_digits_of_jis() {
    let html = render_path("/data/rebar");
    for row in REBAR_ROWS {
        // 1 行は 1 つの tr。セルの文字がこの順で、余計な桁が無い
        let cells: Vec<String> = row.iter().map(|c| format!(">{c}<")).collect();
        let cells: Vec<&str> = cells.iter().map(String::as_str).collect();
        assert_in_order(&html, &cells);
    }
}

/// 種類, 長期 引張・圧縮（D25 以下, D29 以上）, 長期 せん断補強, 短期 引張・圧縮, 短期 せん断補強。
/// RC 規準 表 6.2（`docs/materials/adr/0003` の写し）
const ALLOWABLE_ROWS: [[&str; 6]; 4] = [
    ["SD295", "195", "195", "195", "295", "295"],
    ["SD345", "215", "195", "195", "345", "345"],
    ["SD390", "215", "195", "195", "390", "390"],
    ["SD490", "215", "195", "195", "490", "490"],
];

#[test]
fn rebar_page_shows_allowable_stresses_of_the_four_kinds() {
    let html = render_path("/data/rebar");
    for row in ALLOWABLE_ROWS {
        let cells: Vec<String> = row.iter().map(|c| format!(">{c}<")).collect();
        let cells: Vec<&str> = cells.iter().map(String::as_str).collect();
        assert_in_order(&html, &cells);
    }
    assert!(
        !html.contains(">SD590A<"),
        "表 6.2 に無い種類が行に出ている"
    );
}

#[test]
fn concrete_page_shows_the_unit_weight_table_by_fc_range() {
    let text = text_of(&render_path("/data/concrete"));
    assert_in_order(
        &text,
        &[
            "Fc ≦ 36",
            "23.0",
            "24.0",
            "36 < Fc ≦ 48",
            "23.5",
            "24.5",
            "48 < Fc ≦ 60",
            "24.0",
            "25.0",
            "60 < Fc ≦ 80",
            "24.5",
            "25.5",
            "80 < Fc ≦ 100",
            "25.0",
            "26.0",
        ],
    );
}

#[test]
fn concrete_page_opens_at_fc_24_with_its_results() {
    let html = render_path("/data/concrete");
    assert!(
        html.contains("value=\"24\""),
        "Fc の初期値が 24 でない: {html}"
    );
    assert_in_order(
        &text_of(&html),
        &[
            "23.0",
            "24.0",
            "2.27×10⁴",
            "E = 3.35×10⁴ × (γ/24)² × (Fc/60)^(1/3)",
        ],
    );
}

/// 計算できたときの表示。(γ, γ₁, E, ワーニング)
fn computed(input: &str) -> (String, String, String, Option<String>) {
    match ConcreteReadout::from_fc_input(input) {
        ConcreteReadout::Computed {
            unit_weight,
            rc_unit_weight,
            young_modulus,
            warning,
            ..
        } => (unit_weight, rc_unit_weight, young_modulus, warning),
        unusable => panic!("入力「{input}」が計算できない: {unusable:?}"),
    }
}

#[test]
fn readout_shows_allowable_stresses_to_three_figures() {
    // Fc 24: 圧縮 8, 16 / せん断 0.73, 1.095 / 上端筋 1.54, 2.31 / その他 2.31, 3.465
    let ConcreteReadout::Computed {
        allowable_stresses, ..
    } = ConcreteReadout::from_fc_input("24")
    else {
        panic!("Fc 24 が計算できない");
    };
    let rows: Vec<_> = allowable_stresses
        .iter()
        .map(|r| (r.name, r.long_term.as_str(), r.short_term.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            ("圧縮", "8.00", "16.0"),
            ("せん断", "0.730", "1.10"),
            ("付着（上端筋）", "1.54", "2.31"),
            ("付着（その他の鉄筋）", "2.31", "3.47"),
        ]
    );
}

#[test]
fn readout_rounds_unit_weights_to_one_decimal_and_e_to_three_figures() {
    for (fc, gamma, gamma1, e) in [
        ("24", "23.0", "24.0", "2.27×10⁴"),
        ("60", "24.0", "25.0", "3.35×10⁴"),
        ("100", "25.0", "26.0", "4.31×10⁴"),
    ] {
        assert_eq!(
            computed(fc),
            (gamma.into(), gamma1.into(), e.into(), None),
            "Fc {fc}"
        );
    }
}

#[test]
fn readout_warns_outside_18_to_100_but_still_computes() {
    let (gamma, _, _, warning) = computed("12");
    assert_eq!(gamma, "23.0");
    assert_eq!(warning.as_deref(), Some("Fc が下限 18 N/mm² に満たない"));
    let (gamma, _, _, warning) = computed("120");
    assert_eq!(gamma, "25.0");
    assert_eq!(
        warning.as_deref(),
        Some("Fc が単位体積重量の表の上限 100 N/mm² を超える。γ は最後の区分の値を使う")
    );
}

#[test]
fn readout_is_unusable_with_an_error_when_fc_cannot_be_used() {
    for (input, error) in [
        ("", "Fc を入れる"),
        ("abc", "数値で入れる"),
        ("0", "Fc が正の有限値でない"),
        ("-5", "Fc が正の有限値でない"),
    ] {
        assert_eq!(
            ConcreteReadout::from_fc_input(input),
            ConcreteReadout::Unusable {
                error: error.into()
            },
            "入力「{input}」"
        );
    }
}
