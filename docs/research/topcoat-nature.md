# topcoat の実像と既存 crate 共存性の調査 (T1)

- 対象: `tokio-rs/topcoat` (https://github.com/tokio-rs/topcoat)
- 関連: GitHub issue #2 (parent #1 Wayfinder)
- 調査日: 2026-07-27
- 方針: 一次情報(リポジトリの Cargo.toml / README / examples / crates.io API)に基づく事実の列挙のみ。採否判断は下さない。断定できない点は「不明」と明記。

> 注: 一次ソースは WebFetch 経由で取得。GitHub の raw ファイル/ツリー、および crates.io API を参照した。verbatim 全文取得はツール制約で不可のため、各ファイルから抽出した事実を記載する。ローカルの `matsumok/rustruct` 側は worktree のファイルを直接参照。

---

## 依存の実体

- topcoat は **axum に依存しない**。ワークスペース root `Cargo.toml` の依存に axum は現れず、Web スタックは **hyper 1 / tower 0.5 / tower-http 0.7 / hyper-util 0.1 / http 1.4 / http-body(-util)** を直接使う。ランタイムは **tokio 1.51.1**。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/Cargo.toml
- topcoat は単一 crate ではなく **~31–32 crate のワークスペース**(`topcoat-core`, `topcoat-router`, `topcoat-view`, `topcoat-runtime`, `topcoat-session`, `topcoat-ui-registry`, `topcoat-tailwind`, `topcoat-mail` 等 + それぞれの `*-macro`)。利用者は facade crate `topcoat` を1つ依存に入れ、feature でサブ crate を取り込む構成。
  - facade `crates/topcoat` の直接依存: **tokio(optional, `serve` feature で有効), serde, serde_urlencoded, futures-util, inventory**。hyper/tower は facade の直接依存には出ず、`tower` feature が `topcoat-router/tower` を有効化する(=tower 連携は optional でサブ crate 側)。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/crates/topcoat/Cargo.toml
- **既存 `rustruct-server`(axum 0.8.9 + tokio 1.52.3) との関係**: topcoat は自前のサーバ(`topcoat::start()` / `serve` feature)を持つ**フルスタックフレームワークであり、axum の代替**。HTTP アプリ層としては axum ハンドラを**置き換える**性格。両者とも tokio + tower + hyper の上に載るため下位ランタイム層は共通。
  - **同一プロセス内で axum と topcoat を合成(mount)できるか**は、参照した一次ソースに記述が見当たらず **不明**(両者とも tower Service 系であり理論上は合成余地があるが、topcoat 側にその手順のドキュメント確認はできていない)。
- ローカル比較(出典: worktree `rustruct-server/Cargo.toml`): `axum = "0.8.9"`, `tokio = "1.52.3"(full)`, `sqlx = "0.9.0"(sqlite, runtime-tokio)`, dev-dep に `tower = "0.5.3"`。tokio/tower の系列は topcoat と近接(tower 0.5 系一致、tokio は 1.5x 系一致)。

## レンダリングモデル

- **SSR(サーバサイドレンダリング)が主**。全マークアップをサーバで描画し、コンポーネントは async でサーバ上から直接 DB を参照できる。**クライアント WASM は不要/使わない**(依存に wasm-bindgen 等の WASM 関連は無し)。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/README.md , https://raw.githubusercontent.com/tokio-rs/topcoat/main/Cargo.toml
- **クライアント反応性 `$(...)`**: 型チェックされた通常の Rust 式。サーバは初期描画時にこれを評価し、**同じ式を JavaScript に変換**してブラウザ側でも再実行させる(サーバ評価 + JS へのトランスパイルの二重コンパイル)。
- **ビルド生成物**: 反応性ランタイムは **生成された JavaScript** で動く(WASM ではない)。ランタイムは signals, `$(...)` 式, `@` イベントハンドラ, `:` バインド属性を提供。別建ての明示的ビルドステップ無しで JS を出力する、と README は説明。
- **island か**: 用語としての「island」表現は確認できず。モデルは「SSR + サーバ評価式を JS にトランスパイルしてブラウザで再実行」。`#[shard]` コンポーネントは、その `$(...)` 引数が変わるとサーバ側で再描画される単位。**厳密な island 分類は不明**(部分再描画の仕組みは shard/procedure として存在)。
- **ルーティング**: 手動(`Router::builder()`)と、モジュール構成からの自動推論(例 `app/posts/id.rs` → `/posts/{post_id}`)の2方式。route は `#[page("/...")]` 属性付き async 関数。
- **フォーム/サーバアクション**: `#[shard]`(サーバ再描画)と **procedure(ブラウザから呼べる async サーバ関数)**、および input イベントハンドラによる再描画で扱う。
  - 出典: README(同上), https://raw.githubusercontent.com/tokio-rs/topcoat/main/examples/hello-world/src/main.rs

## 既存 Rust 型の再利用 (rustruct-types)

- topcoat facade は **serde 1 と serde_urlencoded 0.7 を直接依存**に持ち、フォーム/クエリのパースに serde を使う。`rustruct-types` は serde derive のみ依存。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/crates/topcoat/Cargo.toml , worktree `rustruct-types/Cargo.toml`
- 従って **`rustruct-types` の serde derive 型を topcoat のページ/コンポーネント/サーバ関数で共有可能**(同じ serde 1 系エコシステム)。topcoat 例(`toasty-todo`)も `serde(derive)` を利用。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/examples/toasty-todo/Cargo.toml
- 制約: topcoat の `view!` / コンポーネント固有の trait 実装が型に必要になるか(表示用に追加の derive/trait が要るか)は一次ソースで未確認 = **その点は不明**。素の serde 型の受け渡し(フォーム入力・procedure 引数/戻り値)は serde 経由で成立する見込み。

## 永続化 (sqlx 0.9 / SQLite)

- topcoat は **DB ライブラリを同梱・強制しない(database-agnostic)**。README は「コンポーネントはサーバ上で DB を直接クエリできる」とのみ述べ、特定 ORM を規定しない。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/README.md
- topcoat の**公式 DB 例は `toasty`(tokio-rs 製 ORM)with `sqlite` feature** を使用。**sqlx は topcoat のどの依存にも現れない**。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/examples/toasty-todo/Cargo.toml , https://raw.githubusercontent.com/tokio-rs/topcoat/main/Cargo.toml
- 既存の **sqlx 0.9(sqlite, runtime-tokio)をサーバ側でそのまま使えるか**: topcoat は tokio 上で動くため、sqlx(runtime-tokio)を**通常の依存として同居させること自体は技術的に整合**する(ランタイム一致)。ただし topcoat 側に **sqlx 用の一次統合・ドキュメント・例は存在しない**(公式路線は toasty)。「そのまま使える」ことの実証は一次ソースには無く、**アプリ側で自前配線が必要(未実証)**。

## 成熟度 / リスク

- **バージョン**: `0.4.0`(crates.io の max_version = newest_version = 0.4.0)。ワークスペース version も 0.4.0。
  - 出典: https://crates.io/api/v1/crates/topcoat (API), https://raw.githubusercontent.com/tokio-rs/topcoat/main/Cargo.toml
- **crates.io / docs.rs**: crate `topcoat` は**公開済み**。created_at 2026-04-17、updated_at 2026-07-22、公開バージョン計 13、総 DL 約 1,893、yank 無し、カテゴリ web-programming / http-server。
  - 出典: https://crates.io/api/v1/crates/topcoat
- **最終コミット**: main の最新コミットは **2026-07-27**(調査当日)。直近は `docs: improve readme`, `docs(mail): mail example and guide (#221)`, `feat(datastar): add datastar support (#219)` 等。**活発に開発中**。
  - 出典: https://github.com/tokio-rs/topcoat/commits/main
- **成熟度警告**: README/リポジトリ説明に **「early-stage and experimental. Expect breaking changes.(初期段階・実験的、破壊的変更を想定)」** と明記。0.x のため semver 上も破壊的変更が頻発しうる。breaking の具体的度合い(過去リリース間の破壊内容)は個別 CHANGELOG 未確認 = その粒度は **不明**。
- **MSRV**: `rust-version = 1.95`。**edition 2024**。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/Cargo.toml
  - 比較: rustruct 各 crate も edition 2024。MSRV 1.95 は比較的新しめだが整合可能。
- **ライセンス**: **MIT**。
  - 出典: リポジトリ root(LICENSE), https://github.com/tokio-rs/topcoat
- **ドキュメント量**: README から Getting Started / `view!` マクロ / `#[component]` / Router / request context / 統合(Tailwind, htmx, Alpine AJAX, Datastar)等のガイドがリンク。examples は **24 個**(下記)。star 約 3.4k、コミット約 818。
  - 出典: README, https://github.com/tokio-rs/topcoat , https://github.com/tokio-rs/topcoat/tree/main/examples
- **examples 一覧(24)**: alpine-ajax, app-context, asset, context, cookie, datastar, font, hello-world, htmx, icon, mail, manual-router, module-router, path-query-params, procedure, request-response, runtime, session, shard, sse, tailwind, toasty-todo, ui, websocket。
  - 出典: https://github.com/tokio-rs/topcoat/tree/main/examples

## 最小往復 ("Hello + フォーム + サーバ保存往復")

一次ソース(examples/README)から読み取れる最小構成:

- **Cargo 依存(アプリ crate)**:
  - `topcoat`(facade。サーバ起動には `serve` feature、tower 連携が要れば `tower` feature)
  - `tokio`(features: `rt-multi-thread`, `macros`)
  - `serde`(feature `derive`) — フォーム/procedure の入出力型
  - 保存用に DB crate 1つ: topcoat 公式路線は `toasty`(feature `sqlite`)。既存資産を使うなら `sqlx 0.9`(sqlite, runtime-tokio)を自前配線(統合は非公式)。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/examples/hello-world/Cargo.toml , https://raw.githubusercontent.com/tokio-rs/topcoat/main/examples/toasty-todo/Cargo.toml
- **ファイル構成**: 最小は `Cargo.toml` + `src/main.rs`。
  - `#[tokio::main]` の main 内で `topcoat::start(Router::builder().discover().build())` を呼びサーバ起動。`.discover()` が注釈付き関数(ページ)を自動収集。
  - Hello: `#[page("/")]` 付き async 関数が `view!` で HTML を返す。
  - フォーム/往復: フォーム送信は procedure(ブラウザから呼ぶ async サーバ関数)または `#[shard]` の入力イベント再描画で受け、serde `Deserialize` 型で入力を受け取り、サーバ関数内で DB へ保存 → 再描画で反映。
  - 出典: https://raw.githubusercontent.com/tokio-rs/topcoat/main/examples/hello-world/src/main.rs , README(procedure/shard), examples `request-response` / `procedure` / `toasty-todo`
- **未確認点**: `request-response` / `hello-world` の src 全文はツール制約で取得できず、フォーム POST の正確な API 形(procedure シグネチャ、フォーム extractor の型)は **一次ソース上で最終確認できていない = その細部は不明**。構造(page/procedure/shard + serde 型 + DB 書き込み)は README と Cargo.toml から確定。

---

## 出典 URL 一覧

- リポジトリ: https://github.com/tokio-rs/topcoat
- root Cargo.toml: https://raw.githubusercontent.com/tokio-rs/topcoat/main/Cargo.toml
- README: https://raw.githubusercontent.com/tokio-rs/topcoat/main/README.md
- facade crate Cargo.toml: https://raw.githubusercontent.com/tokio-rs/topcoat/main/crates/topcoat/Cargo.toml
- examples 一覧: https://github.com/tokio-rs/topcoat/tree/main/examples
- hello-world main.rs: https://raw.githubusercontent.com/tokio-rs/topcoat/main/examples/hello-world/src/main.rs
- hello-world Cargo.toml: https://raw.githubusercontent.com/tokio-rs/topcoat/main/examples/hello-world/Cargo.toml
- toasty-todo Cargo.toml: https://raw.githubusercontent.com/tokio-rs/topcoat/main/examples/toasty-todo/Cargo.toml
- commits(最終コミット日): https://github.com/tokio-rs/topcoat/commits/main
- crates.io API: https://crates.io/api/v1/crates/topcoat
- (比較)ローカル rustruct: `rustruct-server/Cargo.toml`, `rustruct-types/Cargo.toml`, `rustruct-ui/Cargo.toml`, `CONTEXT.md`
