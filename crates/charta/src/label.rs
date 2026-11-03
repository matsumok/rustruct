//! 線の上の数値（`docs/adr/0011`）。等幅の小さな文字を `ink` で書き、`paper` の縁で白抜きにする。
//! 線に沿って傾け、上下が逆にならないように ±90° に収める。置く場所の候補は図ごとに決め（分布図は
//! 中心からの十字、等厚曲線の図は対角線）、ここは候補を順に、図の中でぶつからないものだけ残す。
//! 図の上の印や文字にかかる候補は、同じ線の上の別の場所を試せる（[`place_groups`]）。

use crate::scene::{Anchor, Font, Item, Paint, Pt, Stroke};

/// 線の上の数値の文字の大きさ。文字の段（`label`・`body`）の外で、この数値だけに使う。`docs/adr/0011`。
pub const LABEL_SIZE: f64 = 9.0;
/// 白抜きの縁の太さ。線の上の数値と、図に重ねる名前や要約の文字に使う。`docs/adr/0011`。
pub const HALO_WIDTH: f64 = 2.5;

/// 置く候補。`at` は文字の中心、`angle` は線の向き（度）。
pub(crate) struct Candidate {
    pub at: Pt,
    pub angle: f64,
    pub text: String,
}

/// 置いた数値。`angle` は ±90° に収めた後の度。
pub(crate) struct Label {
    at: Pt,
    angle: f64,
    text: String,
    /// 縁込みの大きさ（幅、高さ）。ぶつかりの判定に使う
    size: [f64; 2],
}

impl Label {
    fn new(c: Candidate) -> Self {
        Self {
            at: c.at,
            angle: upright(c.angle),
            size: [
                Font::Mono.width(&c.text, LABEL_SIZE) + HALO_WIDTH,
                LABEL_SIZE + HALO_WIDTH,
            ],
            text: c.text,
        }
    }

    /// 回転した矩形の 4 隅。
    fn corners(&self) -> [Pt; 4] {
        let (s, c) = self.angle.to_radians().sin_cos();
        let (hw, hh) = (self.size[0] / 2.0, self.size[1] / 2.0);
        [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]]
            .map(|[u, v]| [self.at[0] + u * c - v * s, self.at[1] + u * s + v * c])
    }

    /// 白抜きの文字。
    pub fn into_item(self) -> Item {
        Item::Text {
            x: self.at[0],
            y: self.at[1],
            text: self.text,
            size: LABEL_SIZE,
            font: Font::Mono,
            fill: Paint::role("ink"),
            anchor: Anchor::Middle,
            rotate: self.angle,
            halo: Some(Stroke::new("paper", HALO_WIDTH)),
        }
    }
}

/// 候補を並びの順に、矩形 `bounds`（左上と右下）に収まり、矩形 `clear` とも先に置いたものともぶつからない
/// ものだけ置く。
pub(crate) fn place(
    candidates: impl IntoIterator<Item = Candidate>,
    bounds: [Pt; 2],
    clear: &[[Pt; 2]],
) -> Vec<Label> {
    place_groups(candidates.into_iter().map(|c| vec![c]), bounds, clear)
}

/// 候補の組を並びの順に置く。組の最初の候補が置ければそれを置く。最初の候補が矩形 `bounds`（左上と右下）
/// からはみ出すか矩形 `clear` にぶつかるときは、組の残り（同じ線の上の別の場所）を順に試し、どれにも
/// ぶつからない最初の 1 つを置く。最初の候補が先に置いた数値とぶつかるときは、組ごと置かない（ぶつかる
/// 数値は並びの先のものを残す）。組の残りがどれも置けないときも置かない。
pub(crate) fn place_groups(
    groups: impl IntoIterator<Item = Vec<Candidate>>,
    bounds: [Pt; 2],
    clear: &[[Pt; 2]],
) -> Vec<Label> {
    let [lo, hi] = bounds;
    let clear: Vec<[Pt; 4]> = clear
        .iter()
        .map(|[a, b]| [*a, [b[0], a[1]], *b, [a[0], b[1]]])
        .collect();
    let free = |corners: &[Pt; 4]| {
        corners
            .iter()
            .all(|p| p[0] >= lo[0] && p[0] <= hi[0] && p[1] >= lo[1] && p[1] <= hi[1])
            && clear.iter().all(|c| !overlaps(corners, c))
    };
    let mut placed: Vec<Label> = Vec::new();
    for group in groups {
        let mut group = group.into_iter().map(Label::new);
        let Some(first) = group.next() else {
            continue;
        };
        let apart = |label: &Label, placed: &[Label]| {
            let corners = label.corners();
            placed
                .iter()
                .all(|other| !overlaps(&corners, &other.corners()))
        };
        let chosen = if free(&first.corners()) {
            apart(&first, &placed).then_some(first)
        } else {
            group.find(|label| free(&label.corners()) && apart(label, &placed))
        };
        placed.extend(chosen);
    }
    placed
}

/// 案内線 `a`→`b` と線分 `p`→`q` の交点。案内線の上の位置 `u`（0 で `a`、1 で `b`）、点、線分の向き（度）。
pub(crate) fn cross(a: Pt, b: Pt, p: Pt, q: Pt) -> Option<(f64, Pt, f64)> {
    let r = [q[0] - p[0], q[1] - p[1]];
    let g = [b[0] - a[0], b[1] - a[1]];
    let den = r[0] * g[1] - r[1] * g[0];
    if den.abs() < 1e-12 {
        return None;
    }
    let t = ((a[0] - p[0]) * g[1] - (a[1] - p[1]) * g[0]) / den;
    let u = ((a[0] - p[0]) * r[1] - (a[1] - p[1]) * r[0]) / den;
    if !(0.0..1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
        return None;
    }
    let at = [p[0] + t * r[0], p[1] + t * r[1]];
    Some((u, at, r[1].atan2(r[0]).to_degrees()))
}

/// 角度を [−90°, 90°) に収める（文字が逆さにならない。縦の線では下から上へ読む）。
fn upright(angle: f64) -> f64 {
    let mut a = angle;
    while a >= 90.0 {
        a -= 180.0;
    }
    while a < -90.0 {
        a += 180.0;
    }
    a
}

/// 2 つの凸四角形が重なるか（分離軸）。
fn overlaps(a: &[Pt; 4], b: &[Pt; 4]) -> bool {
    let axes = |r: &[Pt; 4]| {
        [
            [r[1][0] - r[0][0], r[1][1] - r[0][1]],
            [r[3][0] - r[0][0], r[3][1] - r[0][1]],
        ]
    };
    for axis in axes(a).into_iter().chain(axes(b)) {
        let project = |r: &[Pt; 4]| {
            r.iter()
                .map(|p| p[0] * axis[0] + p[1] * axis[1])
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                    (lo.min(v), hi.max(v))
                })
        };
        let (a0, a1) = project(a);
        let (b0, b1) = project(b);
        if a1 < b0 || b1 < a0 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upright_keeps_text_from_being_upside_down() {
        assert_eq!(upright(135.0), -45.0);
        assert_eq!(upright(-135.0), 45.0);
        assert_eq!(upright(90.0), -90.0);
        assert_eq!(upright(-90.0), -90.0);
        assert_eq!(upright(180.0), 0.0);
    }
}
