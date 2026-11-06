# 状態は要素の属性に持ち、クラスは 1 本の文字にして状態の差分を variant で書く

状態（エラー、開いているか、押されているか）で見た目が変わる要素の書き方を、次のように決めた（2026-11-06、#91。UI の設計の見直し 2026-11-03 の候補 5）。今の規則は `docs/design/README.md` の「状態」。

## 決めたこと

- **状態は要素の属性に持ち、クラスは 1 本の文字にする。** 状態の差分だけを variant で書く（`text-muted error:text-ng`）
- **合う ARIA があればそれを使い、Tailwind 組み込みの variant で書く。** 入力欄のエラーは `aria-invalid` と `aria-invalid:`、量の切替は `aria-pressed` と `aria-pressed:`。条件は属性の 1 か所に書く
- **ARIA の無い要素は `data-state` に持ち、`theme.css` の `@custom-variant` で書く**（Sheet の見出しの `error`）。`theme.rs` が `dark` と同じ所に出す。ADR site-0008 で styled 版から持ち帰るとした `data-state` の約束をここで使う
- **`@custom-variant` の名前は Tailwind 組み込みとぶつけない。** 初めに置いた `open` は組み込みの `open:`（`<details>` の `[open]`）を上書きしていたのでやめた
- **任意値の variant（`data-[state=…]:`）は使わない。** `scripts/check-markup.sh` の `-[` の禁止はそのまま
- **状態でない性質で変わる部分は、同じ要素に `class:` を 2 つ書いて分ける。** Dioxus 0.7 は同じ要素の `class:` を空白でつないで出す（`docs/dioxus-0.7.md`）。共通の部分を 1 つ目に、違う部分だけを選ぶ `if` を 2 つ目に書く
- **`scripts/check-classes.sh` は、拾えない形を落とす。** 文字への `{…}` の差し込み、`class:` に変数や式、枝が文字でない `if`、省略形の `class,` と `class }`。部品が受けた `class` をそのまま渡す `class: "{class}"` だけは通す。通すのは、囲む fn の引数に `class` があって、その fn の中で `let class` と束ね直していないときだけ。落とす形と通す形の見本を検査の中で流し、検査そのものが拾い漏らしていないかも見る

## 理由

状態で 1 語だけ違うクラスの文字を 2 本書くと、違いが読み取りにくく、片方だけ直す間違いが起きる。クラスを変数に束ねると、`check-classes.sh` が文字を拾えず、生成した `tailwind.css` にそのクラスが無くても気づけない。属性に状態を持てば、クラスは 1 本になり、検査も全部を拾える。テストもクラス名でなく属性の値で状態を見られる。

ARIA を先に使うのは、読み上げにも同じ状態が要り、属性が 1 つで済むから。`data-state` を ARIA と並べて持つと、同じ状態を 2 か所に書くことになる。
