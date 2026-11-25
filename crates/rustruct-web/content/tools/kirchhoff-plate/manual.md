## 1 仮定と式

### 仮定

- 板は長方形で、x 方向の辺長 $L_x$、y 方向の辺長 $L_y$、板厚 $t$ を持つ。板厚は全面で一様とする
- 板は薄い。中央面に垂直な線は、曲がった後も中央面に垂直でまっすぐなままとし、せん断の変形は無視する
- たわみは板厚に比べて小さい。中央面は伸び縮みしない
- 材料は線形弾性で等方とし、ヤング係数 $E$ とポアソン比 $\nu$（$0 \le \nu < 0.5$）で表す
- 荷重は全面に一様に載る等分布荷重 $p$ だけとする。自重は導かないので、含めるときは $p$ に足す
- 座標は隅を原点とし、$x$ は $0$ から $L_x$、$y$ は $0$ から $L_y$

### 曲げ剛性とたわみの式

板の単位幅あたりの曲げ剛性 $D$ は次の式で求める。

$$D = \frac{E t^3}{12\left(1 - \nu^2\right)}$$

たわみ $w$ は次の式と、辺ごとの支持条件から決まる。

$$D \nabla^4 w = D \left( \frac{\partial^4 w}{\partial x^4} + 2 \frac{\partial^4 w}{\partial x^2 \partial y^2} + \frac{\partial^4 w}{\partial y^4} \right) = p$$

### 結果に出る量

| 量 | 記号 | 単位 |
|---|---|---|
| たわみ | $w$ | mm |
| 曲げモーメント | $M_x$、$M_y$ | kN·m/m |
| せん断力 | $Q_x$、$Q_y$ | kN/m |
| 辺の反力 | $V_x$、$V_y$ | kN/m |
| 隅反力 | $R$ | kN |

たわみ $w$ は板の中央面に直交する変位。

曲げモーメント $M_x$ は $x$ に直交する断面に働く単位幅あたりの曲げモーメントで、$x$ 方向の応力 $\sigma_x$ を生じさせる。$M_y$ は $y$ について同じ。

$$M_x = -D \left( \frac{\partial^2 w}{\partial x^2} + \nu \frac{\partial^2 w}{\partial y^2} \right)$$

$$M_y = -D \left( \frac{\partial^2 w}{\partial y^2} + \nu \frac{\partial^2 w}{\partial x^2} \right)$$

せん断力 $Q_x$ は $x$ に直交する断面に、$Q_y$ は $y$ に直交する断面に働く単位幅あたりの面外せん断力。

$$Q_x = -D \frac{\partial}{\partial x} \nabla^2 w$$

$$Q_y = -D \frac{\partial}{\partial y} \nabla^2 w$$

ねじりモーメント $M_{xy}$ は辺の反力と隅反力を求めるのに使い、結果には出さない。

$$M_{xy} = D \left(1 - \nu\right) \frac{\partial^2 w}{\partial x \, \partial y}$$

辺の反力 $V_x$ は $x = 0$ と $x = L_x$ の辺の、$V_y$ は $y = 0$ と $y = L_y$ の辺の、辺が板を支える単位長さあたりの力。固定と単純支持の辺に生じ、せん断力とねじりモーメントの変化から求める。

$$V_x = Q_x - \frac{\partial M_{xy}}{\partial y}$$

$$V_y = Q_y - \frac{\partial M_{xy}}{\partial x}$$

隅反力 $R$ は、単純支持の辺どうしの隅と、単純支持の辺と自由辺の隅に集中して生じる力で、隅ごとに 1 つの値を持つ。隅での $M_{xy}$ から求める。固定辺を含む隅と、自由辺どうしの隅では 0。

$$R = 2 M_{xy}$$

それぞれの量の正の向きは節 2 を見る。
