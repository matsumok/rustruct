//! 利用者の座標と解き方の座標の換算（ADR kirchhoff-plate-0002）。利用者の座標は隅が原点の mm、解き方の座標は
//! 代表の向きの板の中央が原点で、x の半辺長 a を 1 とする無次元（y の半辺長は b = Ly'/Lx'）。
//! 向きの写し（x と y の入れ替え、裏返し）と結果の量の写しは写し方（[`Mapping`]、ADR kirchhoff-plate-0003）が持ち、
//! ここはそれを呼んで mm と無次元の間を往き来する。

use rustruct_types::kirchhoff_plate::Edge;

use super::support_type::Mapping;

/// 写し方と利用者の辺長 Lx・Ly [mm] から決まる座標の換算
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Frame {
    mapping: Mapping,
    lx: f64,
    ly: f64,
}

impl Frame {
    /// 写し方と利用者の板の辺長 [mm]
    pub fn new(mapping: Mapping, lx: f64, ly: f64) -> Frame {
        Frame { mapping, lx, ly }
    }

    pub fn mapping(self) -> Mapping {
        self.mapping
    }

    /// 代表の向きの板の x の半辺長 a [mm]。無次元化の長さの単位
    pub fn half_x(self) -> f64 {
        self.representative_lengths().0 / 2.0
    }

    /// 代表の向きの板の y の半辺長 b = Ly'/Lx'（無次元）
    pub fn b(self) -> f64 {
        let (rep_lx, rep_ly) = self.representative_lengths();
        rep_ly / rep_lx
    }

    /// 利用者の点（隅が原点、mm）→ 解き方の座標 (ξ, η)
    pub fn solver_point(self, x: f64, y: f64) -> (f64, f64) {
        let (rx, ry) = self.mapping.representative_point(x, y, self.lx, self.ly);
        (self.xi(rx), self.eta(ry))
    }

    /// 利用者の辺と、辺の始点（座標 0 の端）からの距離 [mm] → 代表の向きの辺と、辺に沿う無次元の座標
    /// （x の辺なら η、y の辺なら ξ）
    pub fn solver_edge_position(self, edge: Edge, along: f64) -> (Edge, f64) {
        let (rep_edge, rep_along) = self
            .mapping
            .representative_edge_position(edge, along, self.lx, self.ly);
        let along = match rep_edge {
            Edge::X0 | Edge::XLx => self.eta(rep_along),
            Edge::Y0 | Edge::YLy => self.xi(rep_along),
        };
        (rep_edge, along)
    }

    /// 解き方の座標 (ξ, η) → 利用者の点（隅が原点、mm）。[`Self::solver_point`] の逆。辺長に対する比で戻すので、
    /// 辺の上の点（ξ = ±1、η = ±b）が丸めで板の外や辺のすぐ内に出ない
    pub fn user_point(self, xi: f64, eta: f64) -> (f64, f64) {
        let (rep_lx, rep_ly) = self.representative_lengths();
        let b = self.b();
        self.mapping.user_point(
            rep_lx * (xi + 1.0) / 2.0,
            rep_ly * (eta / b + 1.0) / 2.0,
            self.lx,
            self.ly,
        )
    }

    /// 代表の向きの板の辺長 (Lx', Ly') [mm]
    fn representative_lengths(self) -> (f64, f64) {
        self.mapping.representative_lengths(self.lx, self.ly)
    }

    /// 代表の向きの板の隅原点の x [mm] → 中央原点の無次元の x
    fn xi(self, x: f64) -> f64 {
        x / self.half_x() - 1.0
    }

    fn eta(self, y: f64) -> f64 {
        (y - self.representative_lengths().1 / 2.0) / self.half_x()
    }
}

/// 換算の往きと帰り。8 通りの写し方で、点と辺の上の位置が元に戻ること、辺の上の点が帰りで板の外に出ないこと
#[cfg(test)]
mod tests {
    use super::*;

    /// 割り切れない辺長。丸めが出やすい
    const LX: f64 = 1234.567;
    const LY: f64 = 3456.789;

    fn frames() -> impl Iterator<Item = Frame> {
        Mapping::ALL.into_iter().map(|m| Frame::new(m, LX, LY))
    }

    fn assert_close(actual: (f64, f64), expected: (f64, f64), what: &str) {
        let tolerance = 1e-12 * LX.max(LY);
        assert!(
            (actual.0 - expected.0).abs() <= tolerance
                && (actual.1 - expected.1).abs() <= tolerance,
            "{what}: {actual:?} と {expected:?}"
        );
    }

    #[test]
    fn half_lengths_follow_the_representative_orientation() {
        let frame = Frame::new(Mapping::IDENTITY, 2000.0, 3000.0);
        assert_eq!((frame.half_x(), frame.b()), (1000.0, 1.5));
        let transposed = Frame::new(
            Mapping {
                transpose: true,
                flip_x: true,
                flip_y: false,
            },
            2000.0,
            3000.0,
        );
        assert_eq!(transposed.half_x(), 1500.0);
        assert!((transposed.b() - 2.0 / 3.0).abs() < 1e-15);
    }

    #[test]
    fn a_transposed_and_flipped_point_lands_where_worked_out() {
        // 2000 × 3000 の板を入れ替えて x を裏返す。代表の向きは 3000 × 2000、a = 1500、b = 2/3。
        // 利用者の (500, 1000) は入れ替えて (1000, 500)、x を裏返して (2000, 500)。ξ = 2000/1500 − 1、
        // η = (500 − 1000)/1500
        let frame = Frame::new(
            Mapping {
                transpose: true,
                flip_x: true,
                flip_y: false,
            },
            2000.0,
            3000.0,
        );
        let (xi, eta) = frame.solver_point(500.0, 1000.0);
        assert!((xi - 1.0 / 3.0).abs() < 1e-15, "ξ = {xi}");
        assert!((eta + 1.0 / 3.0).abs() < 1e-15, "η = {eta}");
    }

    #[test]
    fn a_point_comes_back_from_the_solver_coordinates() {
        for frame in frames() {
            for i in 0..=6 {
                for j in 0..=6 {
                    let (x, y) = (LX * i as f64 / 6.0, LY * j as f64 / 6.0);
                    let (xi, eta) = frame.solver_point(x, y);
                    assert!(
                        (-1.0 - 1e-12..=1.0 + 1e-12).contains(&xi)
                            && (eta.abs() <= frame.b() * (1.0 + 1e-12)),
                        "{:?} の ({x}, {y}) が ({xi}, {eta})",
                        frame.mapping()
                    );
                    assert_close(
                        frame.user_point(xi, eta),
                        (x, y),
                        &format!("{:?} の ({x}, {y})", frame.mapping()),
                    );
                }
            }
        }
    }

    /// 解き方の座標で、代表の向きの辺 `edge` の上の、辺に沿う座標 `along` の点
    fn on_solver_edge(frame: Frame, edge: Edge, along: f64) -> (f64, f64) {
        match edge {
            Edge::X0 => (-1.0, along),
            Edge::XLx => (1.0, along),
            Edge::Y0 => (along, -frame.b()),
            Edge::YLy => (along, frame.b()),
        }
    }

    /// 利用者の辺 `edge` の上の、始点からの距離 `along` の点
    fn on_user_edge(edge: Edge, along: f64) -> (f64, f64) {
        match edge {
            Edge::X0 => (0.0, along),
            Edge::XLx => (LX, along),
            Edge::Y0 => (along, 0.0),
            Edge::YLy => (along, LY),
        }
    }

    #[test]
    fn a_position_on_an_edge_comes_back_to_the_same_edge() {
        for frame in frames() {
            for edge in Edge::ALL {
                let length = match edge {
                    Edge::X0 | Edge::XLx => LY,
                    Edge::Y0 | Edge::YLy => LX,
                };
                for i in 0..=6 {
                    let along = length * i as f64 / 6.0;
                    let (rep_edge, s) = frame.solver_edge_position(edge, along);
                    assert_eq!(rep_edge, frame.mapping().edge(edge));
                    let (xi, eta) = on_solver_edge(frame, rep_edge, s);
                    let (x, y) = frame.user_point(xi, eta);
                    let what = format!("{:?} の {edge} の {along}", frame.mapping());
                    assert_close((x, y), on_user_edge(edge, along), &what);
                    // 辺に直交する座標は辺の上にちょうど戻る
                    match edge {
                        Edge::X0 => assert_eq!(x, 0.0, "{what}"),
                        Edge::XLx => assert_eq!(x, LX, "{what}"),
                        Edge::Y0 => assert_eq!(y, 0.0, "{what}"),
                        Edge::YLy => assert_eq!(y, LY, "{what}"),
                    }
                }
            }
        }
    }

    #[test]
    fn a_point_on_an_edge_stays_on_the_plate() {
        // 解き方の座標で辺の上の点（ξ = ±1、η = ±b）と隅は、利用者の座標で板の上、辺の上にちょうど来る
        for frame in frames() {
            let b = frame.b();
            for k in 0..=8 {
                let t = -1.0 + 2.0 * k as f64 / 8.0;
                for (xi, eta) in [(-1.0, t * b), (1.0, t * b), (t, -b), (t, b)] {
                    let (x, y) = frame.user_point(xi, eta);
                    let what = format!("{:?} の ({xi}, {eta})", frame.mapping());
                    assert!(
                        (0.0..=LX).contains(&x) && (0.0..=LY).contains(&y),
                        "{what} が ({x}, {y})"
                    );
                    assert!(
                        x == 0.0 || x == LX || y == 0.0 || y == LY,
                        "{what} が辺を外れて ({x}, {y})"
                    );
                }
            }
        }
    }
}
