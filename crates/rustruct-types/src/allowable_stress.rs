//! 許容応力度（言葉は `docs/materials/GLOSSARY.md`、値と出典は `docs/materials/adr/0003`）。

use crate::quantity::Stress;

/// 許容応力度の長期と短期の組。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllowableStress {
    pub long_term: Stress,
    pub short_term: Stress,
}

impl AllowableStress {
    /// 短期を「長期に対する値の `factor` 倍」で決める（表 6.1、表 6.3）
    pub(crate) fn short_term_times(long_term: Stress, factor: f64) -> Self {
        Self {
            long_term,
            short_term: Stress::n_per_mm2(long_term.as_n_per_mm2() * factor),
        }
    }
}
