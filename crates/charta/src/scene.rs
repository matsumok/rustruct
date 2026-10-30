//! 描き手に依存しない小さなシーン: パス・矩形・文字だけ。色は役割名で持ち、描くときにテーマで引く。
//!
//! 中間表現の全体（点の記号、格子の場、座標変換の段）はまだ作らない。分布図に要る 3 種だけ。

/// 画面の座標。x は右、y は下が正（SVG と同じ）。
pub type Pt = [f64; 2];

/// 色の役割名（`ink`、`paper`、`seq-3`）。値は `docs/design/tokens.json` にだけある。
pub type Role = String;

/// 塗りや線の色。役割名そのものか、2 つの役割名の間の色（帯の段を 12 色の間から取るとき）。
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    Role(Role),
    /// `from` から `to` へ OKLab で `t`（0 で `from`、1 で `to`）だけ進んだ色。
    Blend {
        from: Role,
        to: Role,
        t: f64,
    },
}

impl Paint {
    pub fn role(role: &str) -> Self {
        Paint::Role(role.to_string())
    }
}

/// 線の色と太さ。
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub paint: Paint,
    pub width: f64,
}

impl Stroke {
    pub fn new(role: &str, width: f64) -> Self {
        Self {
            paint: Paint::role(role),
            width,
        }
    }
}

/// 書体。tokens.json の `type.families` の 2 つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    Sans,
    Mono,
}

/// 文字の寄せ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// 折れ線（複数）。`closed` なら各折れ線を閉じる。帯は 1 つのパスに小片を全部入れる
    Path {
        subpaths: Vec<Vec<Pt>>,
        closed: bool,
        fill: Option<Paint>,
        stroke: Option<Stroke>,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Option<Paint>,
        stroke: Option<Stroke>,
    },
    /// 文字。`(x, y)` は寄せの基準で、上下の中央。`rotate` は度で、時計回りが正。
    Text {
        x: f64,
        y: f64,
        text: String,
        size: f64,
        font: Font,
        fill: Paint,
        anchor: Anchor,
        rotate: f64,
        /// 白抜き: 文字の周りをこの線で縁取り、下の線と塗りを消す。
        halo: Option<Stroke>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub width: f64,
    pub height: f64,
    pub items: Vec<Item>,
}

impl Scene {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            items: Vec::new(),
        }
    }

    /// `other` の中身を、その原点が `at` に来るようにずらして足す（図の周りに縁を取るときなど）。
    /// `other` の大きさは使わない。
    pub fn place(&mut self, other: Scene, at: Pt) {
        let shift = |p: Pt| [p[0] + at[0], p[1] + at[1]];
        self.items.extend(other.items.into_iter().map(|item| {
            match item {
                Item::Path {
                    subpaths,
                    closed,
                    fill,
                    stroke,
                } => Item::Path {
                    subpaths: subpaths
                        .into_iter()
                        .map(|s| s.into_iter().map(shift).collect())
                        .collect(),
                    closed,
                    fill,
                    stroke,
                },
                Item::Rect {
                    x,
                    y,
                    width,
                    height,
                    fill,
                    stroke,
                } => Item::Rect {
                    x: x + at[0],
                    y: y + at[1],
                    width,
                    height,
                    fill,
                    stroke,
                },
                Item::Text {
                    x,
                    y,
                    text,
                    size,
                    font,
                    fill,
                    anchor,
                    rotate,
                    halo,
                } => Item::Text {
                    x: x + at[0],
                    y: y + at[1],
                    text,
                    size,
                    font,
                    fill,
                    anchor,
                    rotate,
                    halo,
                },
            }
        }));
    }
}

/// 継ぎ目: シーンを別のシーンに置くと、形は変わらず位置だけずれる。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placed_items_keep_their_shape_and_move_by_the_offset() {
        let mut inner = Scene::new(10.0, 10.0);
        inner.items.push(Item::Path {
            subpaths: vec![vec![[0.0, 0.0], [10.0, 5.0]]],
            closed: false,
            fill: None,
            stroke: Some(Stroke::new("ink", 1.0)),
        });
        inner.items.push(Item::Rect {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
            fill: None,
            stroke: None,
        });
        inner.items.push(Item::Text {
            x: 5.0,
            y: 5.0,
            text: "1".into(),
            size: 9.0,
            font: Font::Mono,
            fill: Paint::role("ink"),
            anchor: Anchor::Middle,
            rotate: 30.0,
            halo: None,
        });
        let mut outer = Scene::new(30.0, 30.0);
        outer.place(inner, [8.0, 6.0]);
        assert_eq!((outer.width, outer.height), (30.0, 30.0));
        let Item::Path { subpaths, .. } = &outer.items[0] else {
            panic!("{:?}", outer.items[0])
        };
        assert_eq!(subpaths, &vec![vec![[8.0, 6.0], [18.0, 11.0]]]);
        assert!(matches!(
            outer.items[1],
            Item::Rect {
                x: 9.0,
                y: 8.0,
                width: 3.0,
                height: 4.0,
                ..
            }
        ));
        assert!(matches!(
            outer.items[2],
            Item::Text {
                x: 13.0,
                y: 11.0,
                rotate: 30.0,
                ..
            }
        ));
    }
}
