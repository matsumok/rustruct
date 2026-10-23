//! 双曲線関数を比の形の部品だけで評価する（ADR-0002）。級数の高次の項では引数が数百になり、
//! cosh・sinh そのものは f64 の範囲を超える。ここにある形はどれも |x| ≤ k で値が 1 以下に収まる。
//! 標準の cosh・sinh・tanh は `clippy.toml` の disallowed-methods で呼べない。

/// tanh k（k ≥ 0）。e^{−2k} だけで書き、大きな k で 1 に収束する
pub fn tanh(k: f64) -> f64 {
    let e = (-2.0 * k).exp();
    (1.0 - e) / (1.0 + e)
}

/// sech² k = 1 / cosh² k（k ≥ 0）
pub fn sech2(k: f64) -> f64 {
    let e = (-2.0 * k).exp();
    4.0 * e / ((1.0 + e) * (1.0 + e))
}

/// cosh x / cosh k（|x| ≤ k）。1 以下
pub fn cosh_ratio(x: f64, k: f64) -> f64 {
    ((x - k).exp() + (-x - k).exp()) / (1.0 + (-2.0 * k).exp())
}

/// sinh x / cosh k（|x| ≤ k）。絶対値 1 以下
pub fn sinh_ratio(x: f64, k: f64) -> f64 {
    ((x - k).exp() - (-x - k).exp()) / (1.0 + (-2.0 * k).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 比べる相手は指数関数で直接書いた定義。引数が小さいうちは溢れない
    fn naive_cosh(x: f64) -> f64 {
        (x.exp() + (-x).exp()) / 2.0
    }
    fn naive_sinh(x: f64) -> f64 {
        (x.exp() - (-x).exp()) / 2.0
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-14 * b.abs().max(1.0)
    }

    #[test]
    fn ratios_match_the_definition_for_moderate_arguments() {
        for k in [0.1, 1.0, 5.0, 20.0] {
            for x in [-k, -0.5 * k, 0.0, 0.3 * k, k] {
                assert!(
                    close(cosh_ratio(x, k), naive_cosh(x) / naive_cosh(k)),
                    "cosh {x} {k}"
                );
                assert!(
                    close(sinh_ratio(x, k), naive_sinh(x) / naive_cosh(k)),
                    "sinh {x} {k}"
                );
            }
            assert!(close(tanh(k), naive_sinh(k) / naive_cosh(k)), "tanh {k}");
            assert!(
                close(sech2(k), 1.0 / (naive_cosh(k) * naive_cosh(k))),
                "sech² {k}"
            );
        }
    }

    #[test]
    fn large_arguments_stay_finite_and_bounded() {
        let k = 1000.0;
        assert_eq!(cosh_ratio(k, k), 1.0);
        assert_eq!(sinh_ratio(k, k), 1.0);
        assert_eq!(sinh_ratio(-k, k), -1.0);
        assert_eq!(cosh_ratio(0.0, k), 0.0);
        assert_eq!(tanh(k), 1.0);
        assert_eq!(sech2(k), 0.0);
        assert!((0.0..=1.0).contains(&cosh_ratio(500.0, k)));
    }
}
