# 骨格と技術スタック（2026-10-14 確定）

`docs/philosophy.md`、`docs/ui-decisions.md`、`docs/design/README.md`（2026-09-30 のデザインシステム）を前提に、2026-10-14 の設計セッションで決めたこと。
なぜそうしたかの経緯は `docs/adr/` にある。ここは「何に決めたか」の一覧。

## 構成

単一の Cargo ワークスペース。

| クレート | 役割 | 依存 |
|---|---|---|
| `rustruct-types` | 共有型。Model と保存形式 | serde のみ |
| `rustruct-calc` | 計算核。I/O も非同期も持たない純粋関数 | `rustruct-types` |
| `rustruct-web` | Dioxus fullstack アプリ。`web` / `server` feature で切替。SSG のプリレンダーもここ | 上2つ、`charta` |
| `charta` | グラフ描画クレート。汎用。`rustruct-types` に依存しない。後で別リポジトリに出せる。設計メモは `docs/ui-decisions.md` のグラフ描画ライブラリの節 | なし |

招待者向けサーバー（旧 `rustruct-server`）は作らない。必要になったら ADR-0002 を書き直してから足す。

## UI

- Dioxus 0.7 系（0.8 は alpha のため見送り）。Web ターゲット
- 全ページを `dx build --ssg` でプリレンダーし、ブラウザで hydration する（ADR-0001）
- ブラウザでしか決まらない値は初回描画で読まず、起動後に `use_effect` で読む
- CSS は Tailwind v4。`dx` 内蔵の単体 CLI を使い、Node は入れない
- 値の源は `docs/design/tokens.json` の一箇所。Tailwind の `@theme` ブロック（色・文字・余白・角丸）は Rust の小さなビルド工程で tokens.json から生成し、手では書かない。`charta` も同じ JSON から色を読む
  - 生成器は `crates/rustruct-web/src/theme.rs` の純粋関数で、`build.rs` が `crates/rustruct-web/theme.css` に書く。`tailwind.css` がそれを `@import` し、dx が `assets/tailwind.css` に出力する。`theme.css` と `assets/tailwind.css` はどちらも生成物だが、`asset!` がコンパイル時にファイルを要求するのでリポジトリに入れる（手で編集しない）
  - 生成した `@theme` は色・文字・余白・角丸の名前空間で Tailwind の既定値（`bg-red-500`、`text-sm`、`p-3.5` など）を消す。役割名のクラスだけが存在し、tokens.json にない値はマークアップに書けない。`--spacing` の既定の目盛りも消えるので、`w-*` / `max-w-*` / `gap-*` の数値も `space-1`〜`space-8` の範囲だけになる
  - `dx serve` は `docs/` を監視しないので、tokens.json を変えたら `.rs` を保存し直すか `dx serve` を再起動する
- ライトとダークを最初から両方作る。色は役割名トークンで指定し、`@custom-variant dark` の1行で切り替える
- コントラスト検査は Rust のテストとして tokens.json を読んで行い、CI で回す。WCAG 比（usage に書かれた 3:1 / 4.5:1）を必須、APCA を警告として出す。最初の一本には含めない
- フォントは IBM Plex Sans JP と Plex Mono。まず Google Fonts CDN
- ナビゲーションは「住所で組む」。上のバーに wordmark、各項目にパスを添える。サイドバーは未決のまま作らない
- Note は Markdown をリポジトリに置き、ビルド時に Rust で HTML にする。数式は KaTeX をライブラリとして使う

## 配信

- Cloudflare Workers Static Assets に配置する（Pages は維持モードのため使わない）
- GitHub Actions で `dx build --ssg` し、wrangler で配置。`main` への push で本番、PR でプレビュー URL
- 公開面にサーバープロセスは置かない。計算はブラウザ内で完結する（ADR-0002）

## 認証と制限

- 認証は Cloudflare Access のみ。アプリにログイン実装は持たない（ADR-0002）
- 登録者: Access の「Everyone + メールのワンタイム PIN」。無料枠 50 人
- root: Access の許可リストに自分のメールを入れた `/root/` パス。骨組みでは中身は空
- 限定データ: `/private/` 配下の静的 JSON を Access で守る。実体は非公開リポジトリ `rustruct-data` に置き、CI がデプロイキーで取得して同梱する

## 保存

- 形式は整数バージョン付き JSON（ADR-0003）
- 骨組みではブラウザ内（localStorage）とファイル（ダウンロード／読込）の2層だけ
- サーバー保存と、知らない人向けの保存は作らない

## 最初の一本

上のバーに wordmark、トップに Tools / Studies / Notes の一覧（名前＋一行＋パス）、各セクションの空ページ。この骨組みが SSG と hydration を通して rustruct.matsumok.com で表示されるところまで。計算機能はゼロ、サイドバーなし。SSG が通らなければ ADR-0001 の (B) に落とす。

## 旧 ADR

`archive/2026-07-egui` ブランチの ADR 0001〜0004 は復元しない。必要になったときに書き直す。
