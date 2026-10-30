//! 列の値の並び → 小数点を揃える右の埋め。

/// 埋めに使う数字幅の空白
pub const FIGURE_SPACE: char = '\u{2007}';

/// 列の値ごとの、小数点の位置を揃えるのに右に足す数字幅の空白の数。等幅で右に揃えると、小数点
/// （小数点の無い数は整数部の右）が縦に揃う。E の数（-1.52E-06）は仮数の小数点で揃え、
/// 指数の部分（E-06）は小数部と同じく右に伸びる。数でない値（「—」）は小数点の無い数として扱う
pub fn point_pads<S: AsRef<str>>(column: &[S]) -> Vec<usize> {
    let rights: Vec<usize> = column.iter().map(|v| right_of_whole(v.as_ref())).collect();
    let widest = rights.iter().copied().max().unwrap_or(0);
    rights.into_iter().map(|right| widest - right).collect()
}

/// 整数部より右の字数（「12.7」は 2、「-1.52E-06」は 7、「1,140」は 0）
fn right_of_whole(value: &str) -> usize {
    let whole = value.find(['.', 'E']).unwrap_or(value.len());
    value[whole..].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorter_fractions_are_padded_to_the_longest() {
        assert_eq!(point_pads(&["12.7", "1.25", "100"]), [1, 0, 3]);
    }

    #[test]
    fn a_column_without_points_is_not_padded() {
        assert_eq!(point_pads(&["1,140", "95", "—"]), [0, 0, 0]);
    }

    #[test]
    fn points_line_up_in_a_column_with_e() {
        let column = ["6.11", "-1.52E-06", "-15,500", "2E-04"];
        let pads = point_pads(&column);
        assert_eq!(pads, [4, 0, 7, 3]);
        // 等幅で右に揃えると、小数点（無い数は整数部の右）が同じ桁に来る
        let width = column
            .iter()
            .zip(&pads)
            .map(|(v, p)| v.chars().count() + p)
            .max()
            .unwrap();
        let points: Vec<usize> = column
            .iter()
            .zip(&pads)
            .map(|(v, p)| {
                let left = width - v.chars().count() - p;
                let whole = v.find(['.', 'E']).unwrap_or(v.len());
                left + v[..whole].chars().count()
            })
            .collect();
        assert!(points.windows(2).all(|w| w[0] == w[1]), "{points:?}");
    }
}
