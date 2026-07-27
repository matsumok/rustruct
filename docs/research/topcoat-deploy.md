# topcoat のビルド/デプロイ適合性（ADR-0003）調査

- 対象: topcoat (tokio-rs/topcoat), ワークスペース version 0.4.0
- 調査日: 2026-07-27
- 方針: 一次情報（README・Cargo.toml・CLI/tailwind ソース）に基づく事実の列挙のみ。判断・推奨はしない。断定できない点は「不明」と明記。
- 参照 ADR: `docs/adr/0003-vertical-slice-first-with-full-deployment-pipeline.md`

---

## ビルドツールチェーン

- **Rust のみで完結する。node / npm / 外部バンドラは不要。** Getting Started は `cargo new` → `cargo add topcoat` / `cargo add tokio --features rt-multi-thread,macros` → `cargo run` のみで起動する（出典: getting_started.md L9-19, L59）。
- **開発補助として CLI `topcoat`（`cargo install topcoat-cli`）** がある。担当は dev サーバ / ソース整形（fmt）/ アセットバンドル（asset）/ UI コンポーネント取得（ui）。`cargo run` 単体でもサーバは起動するため CLI は開発必須ではない（getting_started.md L59-79）。ただし **本番向けアセットバンドルは CLI の `topcoat asset bundle` が担う**（後述）。CLI のサブコマンドは `asset` / `dev` / `fmt` / `ui` のみで、**バイナリをビルドする `build` サブコマンドは存在しない**（バイナリ自体は `cargo build` で作る）（出典: topcoat-cli/src ディレクトリ、cli asset.rs L19-27）。
- **Tailwind を使う場合**: facade の `tailwind` feature を有効化する（README L189-195）。実装は topcoat-tailwind のビルドスクリプトが **standalone Tailwind CLI バイナリ（tailwindlabs/tailwindcss、既定バージョン v4.3.2）を GitHub Releases からビルド時に自動ダウンロード**する方式で、**Node は不要**（出典: topcoat-tailwind/src/build/executable.rs L13-16, L20-31, L69-82）。
  - ダウンロードは `ExecutableSource::Path`（PATH 上の既存 `tailwindcss`）または `ExecutableSource::Env`（環境変数で指すパス）で **回避でき、オフライン/CI では事前配置した tailwindcss バイナリを使える**（executable.rs L32-43）。
  - **既定ではビルド時にネットワーク（GitHub へのアクセス）が必須。** checksum 検証は任意（`sha256:` プレフィックス）。ダウンロード物は target 配下 `topcoat/cache/tailwind/...` にキャッシュされ再利用される（executable.rs L69-110）。
- **Rust バージョン要件は比較的新しい**: `edition = "2024"`, `rust-version = "1.95"`（root Cargo.toml L71, L79）。リポジトリに `rust-toolchain.toml` 同梱。
- **HTTP スタックは hyper 1 / tower 0.5 / tower-http 0.7**（axum ではない）（root Cargo.toml L166-199）。
- **コンテナ/CI に足すことになるもの（事実）**: Rust 1.95+ ツールチェーン。Tailwind 使用時はビルド時に GitHub への到達性、または事前ダウンロードした `tailwindcss` バイナリの配置。アセットに Web フォント/アイコンを含める場合はバンドル時のネットワーク I/O（後述）。**node/Tailwind CLI(npm)/bundler の追加は不要。**

## 成果物

- **コンパイル済みサーバは単一 Rust バイナリ**（`cargo build --release`）。
- **静的アセット（`asset!` で宣言したファイル、Web フォント、アイコン、Tailwind CSS）は実行時に「アセットディレクトリ」から配信される。** 本番用ディレクトリは `topcoat asset bundle` が生成（既定 `<cargo-target>/assets`）。すなわち **本番成果物は「バイナリ + バンドル済み assets ディレクトリ」の 2 点**（出典: README L176-186、cli asset.rs L9, L19-27、cli asset/bundle.rs L11-18, L40-49）。
- `topcoat asset bundle` は「バイナリをビルド→走査し、埋め込まれた `asset!` 参照をディレクトリへ書き出す」。bundler は「filesystem **and network** I/O でブロックする」ため、**バンドル時にアセット（フォント/アイコン等）をネットワークからダウンロードしうる**（bundle.rs L20-24, L40-58）。
- **単一バイナリのみ（assets ディレクトリ無し）で本番稼働が完結するかは、README/docs に明記が無く不明。** README は「local asset directory に copy して Topcoat が配信する」と記述しており、実行時はディレクトリ配信が前提と読める。アセットのバイト列がバイナリにも埋め込まれてディレクトリを不要にできるか否かは確認できず **不明**。
- CDN 等での **別配信は必須ではない**（自前バイナリが content-hash 付き URL で配信、README L176-178, L221-224）。

## 実行モデル

- `topcoat::start(router)` が **tokio/hyper の非同期 HTTP サーバ**を起動（README L34-44、getting_started.md L30-33）。facade の tokio feature は `net`・`signal` を含む（facade Cargo.toml L197）。
- **既定バインドは `127.0.0.1:3000`。環境変数 `HOST` / `PORT` で上書き可**（例 `HOST=0.0.0.0 PORT=8080`）（getting_started.md L59, L83-87）。
- **リッスンするポート**: 既定 3000、任意ポートに変更可。**1024 未満の特権ポートを要求する記述は無い。**
- サーバ自身が **SSR した HTML と静的アセットの双方を配信**（README L64-68, L176-186）。
- **Cloudflare Tunnel（リバースプロキシ）背後**: 動作を妨げる記述は無い。ただしコンテナ/Tunnel から到達させるには既定の `127.0.0.1` ではなく **`HOST=0.0.0.0` が必要**。TLS 終端についての記述は無く、Tunnel 側が担う前提と矛盾しない。`X-Forwarded-*` 等プロキシ経由ヘッダや base URL の扱いに関する公式記述は見当たらず、Tunnel 併用時の細部は **不明**。
- 開発時のライブリロードは `topcoat::dev::script()` を含むページのみ（getting_started.md L42, L79）。本番ページには通常含めない。`topcoat dev` はソース監視・再ビルド・再起動するため **本番実行には使わない**（getting_started.md L71-79）。

## コンテナ制約

- **非root 実行**: バイナリは特権不要のユーザ空間 HTTP サーバ。既定 3000/任意の非特権ポートで listen するため **非root と相性が良い**。root を要求する記述は無い。
- **最小マウント**: 実行時に必要なのは **バイナリ + バンドル済み assets ディレクトリ**（read-only 配布可）。`topcoat dev` はソースツリーを監視・再ビルドするが、本番実行では使わないため **ソースツリーのマウントは不要**。書き込みが必要な領域（セッションストア、DB ファイル等）はアプリ設計依存で **不明**。
- tokio の `signal` feature が有効なため **グレースフルシャットダウンの下地はある**が、`topcoat::start` が SIGTERM を実際にハンドルするかの明示的記述は未確認で **不明**。
- リポジトリに **Dockerfile は無い**（examples 一覧に docker 例なし、root Cargo.toml L37-60）。マルチステージビルド等の公式ガイドも無し。

## 既存運用（sqlx migrate, tracing）併用

- **sqlx migrate**:
  - **topcoat は sqlx に一切依存しない**（root / facade / core の Cargo.toml に sqlx 無し）。
  - DB 層は **非規定**で、コンポーネントは「サーバ側で直接 DB クエリ」できる（README L64-68, L95-105）＝任意の async Rust ライブラリ（sqlx を含む）を利用可能。よって独立 CLI である **`sqlx migrate` はそのまま併用できる**（topcoat がそれを妨げる要素は無い）。
  - ただし topcoat の**サンプル/想定 ORM は Toasty（`toasty = "0.7"`、tokio 製、実験的）**であり sqlx とは別物（root Cargo.toml L194、examples の `toasty-todo`）。**topcoat が sqlx 用の統合やマイグレーションを提供する事実は無く、sqlx 構成は自前で組む必要がある。**
- **tracing**:
  - **topcoat は tracing / tracing-subscriber / log に依存しない**（root / facade / core の Cargo.toml に無し）。
  - topcoat が内部で tracing span/イベントを出力するかは **不明**（依存が無いため出力しない可能性が高いが断定しない）。
  - facade に `tower` feature があり **Tower Service を取得できる**（facade Cargo.toml L151-153）。tower-http の `TraceLayer` 等を自前で挟むことは可能。また通常の tokio アプリ同様、`main()` で `tracing_subscriber` を初期化してアプリ側ログを出すことは可能。**ただし topcoat が標準で tracing を組み込んでいるわけではない。**

## ADR-0003 との衝突点（事実列挙・判断なし）

1. **公式デプロイ/Docker ガイドが存在しない。** README ロードマップで「Docs for how to deploy Topcoat」が **未チェック**（README L257）、リポジトリに Dockerfile・デプロイ手順が無い。ADR-0003 は「Docker コンテナ化→TrueNAS 配置→Cloudflare Tunnel 経由」を一気通貫で要求するが、topcoat 側の公式ガイドは無く自前構築になる。
2. **マイグレーション手段の不一致。** ADR は `sqlx migrate` を明示。topcoat は sqlx 非依存で、サンプル/想定 ORM は Toasty。sqlx 併用は技術的に可能だが、topcoat の機能とは別レイヤであり、topcoat が sqlx マイグレーションを支援する事実は無い。
3. **ログ手段の不一致。** ADR は `tracing` による最小ログ。topcoat は tracing 非依存で、標準では tracing を組み込まない。アプリ側で導入は可能だが「最初から topcoat に乗っている」わけではない。topcoat 内部ログの有無は不明。
4. **成果物が単一バイナリのみで完結しない可能性。** 実行時に assets ディレクトリの同梱・配信が前提と読め（単一バイナリのみで完結するかは不明）、コンテナ化時に `topcoat asset bundle`（＝CLI）をビルドパイプラインへ追加する必要がある。区画化3点セットの「マウント最小化」設計に assets ディレクトリの扱いが加わる。
5. **ビルド時ネットワーク依存。** Tailwind 使用時は既定でビルド時に GitHub Releases から `tailwindcss` バイナリを取得し、アセットバンドル時にもネットワーク I/O が発生しうる。ネットワーク遮断の CI/コンテナビルドでは、バイナリの事前配置（Path/Env）とアセットの事前取得が必要（ADR に直接の記述は無いが再現性・区画化志向と要調整）。
6. **Rust 要件が新しい。** `rust-version = 1.95`、`edition = 2024`。ビルドイメージの Rust を 1.95 以上にする必要がある。
7. **既定バインドが 127.0.0.1。** Tunnel/コンテナから到達させるには `HOST=0.0.0.0` が必須。プロキシ経由のヘッダ/HTTPS 前提の公式記述は無く、Tunnel 併用時の細部は不明。

（上記のうち「衝突」と断定できるのは 1・2・3・4。5・6・7 は追加作業/要調整の事実。）

## 出典 URL 一覧

- README: https://github.com/tokio-rs/topcoat （raw: https://raw.githubusercontent.com/tokio-rs/topcoat/main/README.md ）
- ワークスペース Cargo.toml: https://raw.githubusercontent.com/tokio-rs/topcoat/main/Cargo.toml
- Getting Started: https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/getting_started.md
- facade Cargo.toml: https://raw.githubusercontent.com/tokio-rs/topcoat/main/crates/topcoat/Cargo.toml
- core Cargo.toml: https://raw.githubusercontent.com/tokio-rs/topcoat/main/crates/topcoat-core/Cargo.toml
- Tailwind CLI ダウンロード実装: https://raw.githubusercontent.com/tokio-rs/topcoat/main/crates/topcoat-tailwind/src/build/executable.rs
- CLI asset コマンド: https://raw.githubusercontent.com/tokio-rs/topcoat/main/crates/topcoat-cli/src/asset.rs
- CLI asset bundle: https://raw.githubusercontent.com/tokio-rs/topcoat/main/crates/topcoat-cli/src/asset/bundle.rs
- CLI src ディレクトリ構成: https://github.com/tokio-rs/topcoat/tree/main/crates/topcoat-cli/src
- 参照 ADR（ローカル）: docs/adr/0003-vertical-slice-first-with-full-deployment-pipeline.md

### 補足（到達できなかった一次ソース）
- docs.rs（https://docs.rs/topcoat/...）はプロキシ経由で HTTP 403 のため未取得。
- GitHub REST API（api.github.com）はセッション制約で 403。ファイル取得は raw.githubusercontent.com を使用。
