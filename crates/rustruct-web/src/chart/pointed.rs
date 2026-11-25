//! 指した点（`crates/charta/GLOSSARY.md`）: 図の上の点をマウスで追い、クリック・タップで止める（ADR site-0018）。
//!
//! 図は SVG の文字列のまま、同じ viewBox の層を上に重ねて、十字線だけをその層に描く。点の値は図の下の 1 行で、
//! 値を出すのは図を描いたページ（ここは量の求め方を知らない）。画面の点は、表示の大きさと Scene の大きさの比を
//! 図の矩形から取って Scene の座標に直し、charta の当たり判定（[`Scene::locate`]）に渡す。表示の大きさは
//! ブラウザでしか決まらないので、初回描画では読まない（読むまでは指せない）。

use charta::{Item, Pt, Scene, Stroke};
use dioxus::prelude::*;

use super::svg;
use crate::table::NO_VALUE;

/// 十字線の太さ（ADR site-0018）
const CROSSHAIR_WIDTH: f64 = 0.5;
/// 何も指していないときの点の値の行
pub(crate) const HINT: &str = "図を指すとその点の値を出す";

/// 指した点。当たった描く範囲の番号（`Scene::plots` の何番目か）と、そのデータの点
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pointed {
    pub plot: usize,
    pub at: Pt,
}

/// 図の上で指しているもの。入力や量を変えても保ち、図を描き直したら [`within`](Self::within) で確かめる
/// （置き場は [`use_pointer`]）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Pointer {
    #[default]
    Nothing,
    /// マウスで追っている点
    Following(Pointed),
    /// クリック・タップで止めた点
    Pinned(Pointed),
}

impl Pointer {
    /// 指している点。何も指していなければ無し
    pub fn point(self) -> Option<Pointed> {
        match self {
            Self::Nothing => None,
            Self::Following(p) | Self::Pinned(p) => Some(p),
        }
    }

    /// マウスが図の上で動いた。`hit` は当たり判定の結果で、描く範囲の外なら無し。止めている間は動かさない
    pub fn follow(self, hit: Option<Pointed>) -> Self {
        match self {
            Self::Pinned(_) => self,
            _ => hit.map_or(Self::Nothing, Self::Following),
        }
    }

    /// マウスが図の外へ出た。止めた点は残る
    pub fn leave(self) -> Self {
        match self {
            Self::Pinned(_) => self,
            _ => Self::Nothing,
        }
    }

    /// クリック・タップ。止めていれば外し、止めていなければ `hit` に止める（描く範囲の外なら今のまま）
    pub fn press(self, hit: Option<Pointed>) -> Self {
        match self {
            Self::Pinned(_) => Self::Nothing,
            _ => hit.map_or(self, Self::Pinned),
        }
    }

    /// 描き直した図 `scene` で、指した点がその描く範囲のデータの矩形の中にあるときだけ今のまま。外に出たら
    /// （板を小さくしたときなど）何も指していない
    pub fn within(self, scene: &Scene) -> Self {
        let inside = |p: Pointed| {
            scene.plots.get(p.plot).is_some_and(|plot| {
                (plot.x.0..=plot.x.1).contains(&p.at[0]) && (plot.y.0..=plot.y.1).contains(&p.at[1])
            })
        };
        match self.point() {
            Some(p) if !inside(p) => Self::Nothing,
            _ => self,
        }
    }
}

/// 指しているものの置き場。`within` は今の図で点を確かめる（[`Pointer::within`]）。`within` が読む図が
/// 変わるたびに確かめ、描く範囲の外に出た点は消す（図を戻しても点は戻らない）。点が動いても確かめ直さない
pub(crate) fn use_pointer(mut within: impl FnMut(Pointer) -> Pointer + 'static) -> Signal<Pointer> {
    let mut pointer = use_signal(Pointer::default);
    use_effect(move || {
        let now = *pointer.peek();
        let kept = within(now);
        if kept != now {
            pointer.set(kept);
        }
    });
    pointer
}

/// 画面の点の当たり判定。`offset` は図の矩形の左上からの表示の座標、`shown` は表示の大きさ。表示の大きさと
/// Scene の大きさの比で Scene の座標に直し、当たった描く範囲とデータの点を返す
pub(crate) fn hit(scene: &Scene, offset: Pt, shown: [f64; 2]) -> Option<Pointed> {
    let p = [
        offset[0] * scene.width / shown[0],
        offset[1] * scene.height / shown[1],
    ];
    scene.locate(p).map(|(plot, at)| Pointed { plot, at })
}

/// 十字線の層。図と同じ大きさの Scene に、指した点を通って描く範囲を横切る `ink` の線を縦と横に 1 本ずつ
pub(crate) fn crosshair(scene: &Scene, pointed: Pointed) -> Scene {
    let mut layer = Scene::new(scene.width, scene.height);
    if let Some(plot) = scene.plots.get(pointed.plot) {
        let [lo, hi] = plot.bounds();
        let [x, y] = plot.to_screen(pointed.at);
        layer.items.push(Item::Path {
            subpaths: vec![vec![[lo[0], y], [hi[0], y]], vec![[x, lo[1]], [x, hi[1]]]],
            closed: false,
            fill: None,
            stroke: Some(Stroke::new("ink", CROSSHAIR_WIDTH)),
        });
    }
    layer
}

/// 点の値の 1 項（「x = 2.00 m」）。値は桁を決めた文字で、計算核が出せなければ無し（「—」）
#[derive(Debug, Clone, PartialEq)]
pub struct PointTerm {
    pub symbol: &'static str,
    pub value: Option<String>,
    pub unit: &'static str,
}

/// 指せる図。図の SVG（`svg`、`scene` を描いたもの）の上に十字線の層を重ねる。図の名前は `label`
/// （`role="img"` のまま。キーボードでは指せない）。図の中の要素はポインターを受けず、当たり判定は
/// 図の矩形の座標から `scene` で引く
#[component]
pub(crate) fn PointedFigure(
    label: String,
    svg: String,
    scene: Scene,
    pointer: Signal<Pointer>,
) -> Element {
    let mut shown = use_signal(|| None::<[f64; 2]>);
    // 当たり判定と十字線に要るのは大きさと描く範囲だけ。ポインターが動くたびに描くのは十字線の層だけ
    let mut frame = Scene::new(scene.width, scene.height);
    frame.plots = scene.plots.clone();
    let hit_at = {
        let frame = frame.clone();
        move |offset: Pt| shown().and_then(|shown| hit(&frame, offset, shown))
    };
    let (on_move, on_press) = (hit_at.clone(), hit_at);
    let current = {
        let frame = frame.clone();
        move || pointer.peek().within(&frame)
    };
    let (now_move, now_leave, now_press) = (current.clone(), current.clone(), current);
    rsx! {
        figure {
            class: "relative w-fit",
            role: "img",
            "aria-label": label,
            onresize: move |event: Event<ResizeData>| {
                if let Ok(size) = event.get_content_box_size() {
                    shown.set(Some([size.width, size.height]));
                }
            },
            onpointermove: move |event: Event<PointerData>| {
                if event.pointer_type() == "touch" {
                    return;
                }
                let offset = event.element_coordinates();
                pointer.set(now_move().follow(on_move([offset.x, offset.y])));
            },
            onpointerleave: move |_| pointer.set(now_leave().leave()),
            onclick: move |event: Event<MouseData>| {
                let offset = event.element_coordinates();
                pointer.set(now_press().press(on_press([offset.x, offset.y])));
            },
            div { class: "pointer-events-none", dangerous_inner_html: svg }
            Crosshair { frame, pointer }
        }
    }
}

/// 十字線の層。指していなければ置かない（図の上に重ねるので、置いても図の下はずれない）
#[component]
fn Crosshair(frame: Scene, pointer: Signal<Pointer>) -> Element {
    let layer = pointer()
        .within(&frame)
        .point()
        .map(|pointed| svg(&crosshair(&frame, pointed)));
    rsx! {
        if let Some(layer) = layer {
            div { class: "pointer-events-none absolute inset-0", dangerous_inner_html: layer }
        }
    }
}

/// 点の値の行。図と注記の間に置き、何も指していないときは読み方の文を `muted` で書く。どちらも 1 行で、
/// 行の場所はいつも空いている
#[component]
pub(crate) fn PointValue(terms: ReadSignal<Option<Vec<PointTerm>>>) -> Element {
    rsx! {
        match terms() {
            Some(terms) => rsx! {
                p { class: "flex gap-3 text-body whitespace-nowrap",
                    for term in terms {
                        span {
                            "{term.symbol} = "
                            span { class: "font-mono", {term.value.unwrap_or_else(|| NO_VALUE.to_string())} }
                            " {term.unit}"
                        }
                    }
                }
            },
            None => rsx! {
                p { class: "text-body text-muted whitespace-nowrap", {HINT} }
            },
        }
    }
}

/// 継ぎ目: ポインターの動きと押すことで、指した点が追う・止まる・外れる。画面の点は表示の大きさの比で
/// Scene の座標に直してから当てる
#[cfg(test)]
mod tests {
    use super::*;
    use charta::Plot;

    fn at(x: f64, y: f64) -> Option<Pointed> {
        Some(Pointed {
            plot: 0,
            at: [x, y],
        })
    }

    #[test]
    fn the_mouse_follows_until_a_press_pins_and_a_second_press_releases() {
        let following = Pointer::Nothing.follow(at(1.0, 1.0));
        assert_eq!(following, Pointer::Following(at(1.0, 1.0).unwrap()));
        // 描く範囲の外で消える。図の外へ出ても消える
        assert_eq!(following.follow(None), Pointer::Nothing);
        assert_eq!(following.leave(), Pointer::Nothing);
        // 押すとその点に止まり、動いても外へ出ても残る
        let pinned = following.press(at(2.0, 1.0));
        assert_eq!(pinned, Pointer::Pinned(at(2.0, 1.0).unwrap()));
        assert_eq!(pinned.follow(at(3.0, 1.0)), pinned);
        assert_eq!(pinned.leave(), pinned);
        // もう一度押すと外れる（タップも同じ）
        assert_eq!(pinned.press(at(3.0, 1.0)), Pointer::Nothing);
        assert_eq!(
            Pointer::Nothing.press(at(1.0, 2.0)),
            Pointer::Pinned(at(1.0, 2.0).unwrap())
        );
        // 描く範囲の外を押しても止まらない
        assert_eq!(Pointer::Nothing.press(None), Pointer::Nothing);
        assert_eq!(following.press(None), following);
    }

    /// 0〜4 × 0〜2 を、縁 10 の内側の 200 × 100 に写す図
    fn figure() -> Scene {
        let mut scene = Scene::new(220.0, 120.0);
        scene.plots.push(Plot {
            x: (0.0, 4.0),
            y: (0.0, 2.0),
            origin: [10.0, 10.0],
            size: [200.0, 100.0],
        });
        scene
    }

    #[test]
    fn a_screen_point_is_scaled_by_the_shown_size_before_it_is_located() {
        let scene = figure();
        // 等倍と、半分の大きさに縮めて表示したとき
        assert_eq!(hit(&scene, [110.0, 60.0], [220.0, 120.0]), at(2.0, 1.0));
        assert_eq!(hit(&scene, [55.0, 30.0], [110.0, 60.0]), at(2.0, 1.0));
        // 縁は描く範囲の外
        assert_eq!(hit(&scene, [5.0, 60.0], [220.0, 120.0]), None);
    }

    #[test]
    fn a_point_is_kept_while_it_is_inside_the_plot_of_the_redrawn_figure() {
        let scene = figure();
        let pinned = Pointer::Pinned(at(3.0, 1.5).unwrap());
        assert_eq!(pinned.within(&scene), pinned);
        let mut smaller = figure();
        smaller.plots[0].x = (0.0, 2.5);
        assert_eq!(pinned.within(&smaller), Pointer::Nothing);
        assert_eq!(Pointer::Nothing.within(&scene), Pointer::Nothing);
    }

    /// 図と指しているものの置き場を、試す側に context で渡す
    #[component]
    fn Pointing() -> Element {
        let scene = use_signal(figure);
        let pointer = use_pointer(move |p| p.within(&scene.read()));
        use_context_provider(|| (scene, pointer));
        rsx! {}
    }

    #[test]
    fn a_point_dropped_by_a_smaller_figure_does_not_come_back_when_the_figure_grows_again() {
        let mut dom = VirtualDom::new(Pointing);
        dom.rebuild_in_place();
        let (mut scene, mut pointer) = dom.in_scope(
            ScopeId::APP,
            consume_context::<(Signal<Scene>, Signal<Pointer>)>,
        );
        let pinned = Pointer::Pinned(at(3.0, 1.5).unwrap());
        dom.in_runtime(|| pointer.set(pinned));
        dom.render_immediate_to_vec();
        // 辺長を変えて図を描き直す（x の範囲を 0〜x にする）。描き直したあとの指しているもの
        let mut redraw = |x: f64| {
            dom.in_runtime(|| scene.write().plots[0].x = (0.0, x));
            dom.render_immediate_to_vec();
            dom.in_runtime(|| *pointer.peek())
        };
        assert_eq!(redraw(5.0), pinned);
        assert_eq!(redraw(2.5), Pointer::Nothing);
        assert_eq!(redraw(4.0), Pointer::Nothing);
    }

    #[test]
    fn the_crosshair_crosses_the_plot_through_the_point_in_ink() {
        let layer = crosshair(&figure(), at(1.0, 0.5).unwrap());
        assert_eq!((layer.width, layer.height), (220.0, 120.0));
        let [
            Item::Path {
                subpaths, stroke, ..
            },
        ] = &layer.items[..]
        else {
            panic!("{:?}", layer.items)
        };
        assert_eq!(
            subpaths,
            &vec![
                vec![[10.0, 85.0], [210.0, 85.0]],
                vec![[60.0, 10.0], [60.0, 110.0]]
            ]
        );
        assert_eq!(stroke, &Some(Stroke::new("ink", 0.5)));
    }
}
