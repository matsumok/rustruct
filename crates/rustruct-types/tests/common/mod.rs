/// 単位の換算で生じる2進の誤差だけを許す。表の最下位桁（丸めの幅）よりはるかに小さい
pub fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 1e-12,
        "{what}: {actual} != {expected}"
    );
}
