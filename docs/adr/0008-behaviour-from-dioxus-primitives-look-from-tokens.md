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

## 皮も採る道を検討し直した（2026-10-18）

- 見た目の層（styled 版）も最初から採り、その上にデザインシステムを載せる道を 3 通りで比べた。theme の変数（`--primary-color-1〜7` など）を tokens.json から生成する対応表だけ／写した `style.css` を `var(--color-ink)` の形で書き直す／`css_module` を外して `style.css` を `tailwind.css` から `@import` し `@apply` の役割クラスで書く
- 色は Radix の段で対応表が作れる（Lucide のアイコンと、グレーの段数が増えることは譲れる）。だが角丸 8px、部品の文字 14px、transition と focus の影、hover の塗りと disabled の半透明は譲れないので、対応表だけでは済まず、写した 45 部品の `style.css` を自分で保守することになる。写した後は上流との縁が切れ、もう一度 `dx components add` すると編集が上書きされる
- 持ち物の量、仕組みの数（CSS モジュールか `@import` か）、`rev` を上げたときに確かめる範囲のどれでも骨だけが最小なので、この決定は変えない。皮から持ち帰るのは合成の手順（Combobox は入力・一覧・項目・印）と `data-state` の約束だけで、薄いラッパーに写す
- 骨だけの形が Checkbox と Combobox でも成り立つことは試作で確かめた。役割クラスだけで着せられ、release の WASM は 2,202,475 → 2,767,399 バイト（依存そのものの分。メニューで既に払っている）、SSG と hydration は通った
