# チェックと選択の欄、分布図の量の切替は primitives の部品にする

Sheet のチェックの欄（素の checkbox）と選択の欄（素の select）、Kirchhoff 板の分布図の量の切替（`aria-pressed` のボタン 5 つ）を、`dioxus-primitives` の checkbox・select・toggle_group に置き換えた（2026-11-06、#72）。振る舞いは primitives、見た目は tokens の役割クラス（ADR site-0008）。今の規則は `docs/design/README.md` の「ツール画面の約束」「状態」「図とアイコン」。

## 決めたこと

- **チェックの欄はセルいっぱいのボタンにする。** 素の checkbox はブラウザが描くので、役割クラスで着せられない。primitives の checkbox はボタン（`role="checkbox"`、`aria-checked`）なので、セル全体を押せる器にし、中に `size-4` の箱を描く。箱は書ける器の `line-strong` の枠、オンで `ink` の塗りに `paper` の `checkmark`。印は `size-3`（`space-3`）。`size-4` の箱と同じ大きさだと印が枠に触れるので、段を 1 つ下げて箱の内側に空きを残す。`space-3` の usage にこの用途は無かったので足した（`space-4` にも箱の大きさを足した）。角は既定の `radius-0`（素の checkbox の小さな角丸はやめた）。フォーカスはブラウザの輪でなく、書けるセルと同じ `ink` の内枠
- **選択の欄は、ボタンと選択肢の一覧にする。** ボタンは書けるセルと同じ形で、選んだ名前と開く印。素の select の印はブラウザが描いていたので、メニューの枝と同じ Carbon の `chevron--down` を置く（アイコンを使う場所が増えた）。収まらない名前は素の select と同じく切る
  - 名前はページの値から描き、primitives の `SelectValue` は使わない。選択肢は描いた後の effect で登録されるので、`SelectValue` は SSG の HTML でも hydration の初回でも空の置き場の文字になる
  - 一覧は位置を指定しない `absolute` で置く。置かれる場所はボタンの真下のまま、包む箱は Sheet の外（ページ）になるので、Sheet の角を丸める `overflow-hidden` に切られない。セルや Sheet に `relative` を付けると切られる（Sheet の中に位置を決めるクラスが無いことを render のテストが見る）
  - 名前の左の空きは、数値と文字の欄と同じ `px-2`。素の select では、ブラウザが描く内側の空きが足されて名前が右にずれていた。今は Sheet の欄の文字の左が、欄の種類によらずそろう
  - 閉じたときにフォーカスがまだ一覧の中にあれば、ボタンに戻す（Enter・Space・Esc・選択肢を押して閉じたとき）。primitives はこれをせず、一覧が消えるとフォーカスが `body` に落ちて、読み上げも次の Tab も欄の位置を失う。primitives のボタンは外から `onmounted` を受けないので、一覧を指す `aria-controls` からボタンを探す。探すのとフォーカスを移すのは `web-sys`（ブラウザの WASM だけの依存）で Rust に書き、JS は書かない（`docs/philosophy.md`）。フォーカスを移すのは閉じる処理を抜けてから（Dioxus の `spawn`。primitives も一覧にフォーカスを移すのに同じ形を使う）。処理の中で移すと、フォーカスの抜けた選択肢の blur が入れ子で走り、`RefCell` の二重借用で WASM が止まった
- **量の切替は toggle_group の 1 つだけ押せる形にする。** 押されたものをもう一度押すと primitives は外そうとする（空の組を渡す）ので、空の組は受けない。Tab では組に 1 度だけ止まり、組の中は左右の矢印と Home・End で移る。フォーカスは `focus-visible` の `ink` の内枠（押されたものの上は `paper`）。前は外に描くブラウザの輪で、器の `overflow-hidden` に切られ、`ink` で塗ったものの上では見えなかった
- **状態はどれも ARIA で書く。** `aria-checked`（`group-aria-checked:`）、`aria-selected:`、`aria-pressed:`。primitives が並べて出す `data-state` は使わない（ADR site-0016）

## 理由

素の select と checkbox は振る舞いが揃っているが、見た目の一部（印、角、フォーカスの輪、開いた一覧）をブラウザが描くので、tokens の 1 系統で書けない。primitives なら振る舞い（キーボード、読み上げの役割）を任せたまま、見た目を役割クラスだけで書ける。量の切替は手組みの `aria-pressed` のボタンで、Tab が 5 回止まり、矢印で移れなかった。

## 残ること

- primitives の部品の多くは、選択肢や項目を描いた後の effect で登録する。SSG の HTML と hydration の初回は登録前の姿で、量の切替はどれも `tabindex="-1"` になる。WASM が起きるまでは押しても動かないので、困らない
- 開いた一覧とキーボードの操作は render のテスト（WASM もブラウザも使わない）では見えない。headless Chrome で押して確かめた
