/// 級数の打ち切り誤差を許す比較。`tolerance` は絶対値で、呼ぶ側が値と理由を書く
pub fn assert_within(actual: f64, expected: f64, tolerance: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: {actual} と {expected} の差 {} が許容 {tolerance} を超える",
        (actual - expected).abs()
    );
}
