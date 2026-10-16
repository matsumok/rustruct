# 部品の振る舞いは Dioxus Components の primitives に任せ、見た目は tokens と Tailwind で書く

Dioxus Components（DioxusLabs/components）に当てはまる部品があるものは、それを使う（2026-10-16、本人が決めた）。それまでは比べずに、ヘッダー・メニュー・表・入力欄をすべて手で組んでいた。最初に置き換えたのは #28 のメニュー（dialog と collapsible）。

## 決めたこと

- 使うのは振る舞いの層の `dioxus-primitives` だけ。開け閉め・Esc・外を押して閉じる・フォーカスの閉じ込め・`role` と `aria-*` を部品に任せる
- 見た目の層（`dx components add` が写す `components/` の styled 版、`dx-components-theme.css`、Lucide のアイコン）は入れない。primitives の部品に Tailwind のクラスを渡し、色と大きさは tokens から取る。アイコンは Carbon のまま
- `dioxus-primitives` は crates.io に出ていないので、git の版を `rev` で固定する（`crates/rustruct-web/Cargo.toml`）。上げるときは振る舞いを確かめ直す
- 手組みのまま残っているもの（アカウントの一言、入力欄など）は、当てはまる部品があれば別の Issue で置き換える

## 理由

styled 版は shadcn に倣った見た目で、css_module の CSS とテーマの CSS 変数で組まれている。このサイトの見た目は tokens.json から作る Tailwind の役割名のクラスで書く決まり（ui-decisions）なので、styled 版を入れると見た目の決め事が2系統になる。一方、Esc で閉じる、フォーカスを閉じ込める、読み上げの属性といった振る舞いは、手で組むと抜けやすい。振る舞いだけを借りれば、見た目の決め事を1系統に保ったまま、その抜けを部品に任せられる。

`dioxus-primitives` は 0.0.1 で、依存（`dioxus-sdk-time`、`time`、`palette` など）も増える。版を固定することで、知らないうちに振る舞いが変わらないようにした。
