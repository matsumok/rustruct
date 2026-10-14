# Dioxus 0.7 の勘所

コードから読めないことだけ。vendored ソースを読んで確かめた日付を添える（#11: 2026-10-14、#12: 2026-10-14）。
ここに無いことは `~/.cargo/registry/src/*/dioxus-*-0.7.10/` を grep して確かめる。公式ドキュメントと検索結果は
0.6 の記述が多く、0.7 では当たらない（`server_only!` が無い、`#[server]` の endpoint の形、など）。
CLI（`dx`）の挙動は `dioxus-cli-0.7.10/src/`、fullstack のサーバー側は `dioxus-server-0.7.10/src/`。

- `router` は `dioxus` の既定 feature に無い。`Cargo.toml` で足して初めて `Router` / `Link` / `Routable` が prelude に出る
- `#[route("/tools", SectionPage)]` の第2引数でコンポーネントを指定できる。共有するコンポーネントは `use_route::<Route>()` で自分のルートを知る
- ネイティブ描画は `VirtualDom` に `Rc<dyn History>`（`dioxus::history::MemoryHistory`）と `Rc<dyn Document>` を root context で渡せば動く。`document::Title` / `Meta` は `Document` の `set_title` / `create_head_element` に来る（`crates/rustruct-web/src/prerender.rs`）
- `document::Meta` は初回描画後の props 変更を head に反映しない（SPA 遷移で description は変わらない。SSG の HTML には各ページの値が入る）
- `dx build --ssg` はサーバーを起動し、`POST /api/static_routes` が返す `Vec<String>` のパスを順に取得する。`Route::static_routes()` がその列挙元（引数のあるルート、つまり not-found の catch-all は入らない）。保存はサーバー側の仕事で、`ServeConfig::new().incremental(IncrementalRendererConfig::new().static_dir(public))` が無いと何も書かれない（`crates/rustruct-web/src/ssg.rs`）。書き先は `public/<住所>/index.html` で、`/` は client が書いた殻の `index.html` を上書きする
- client と server のビルドは既定で並行に走り、server の SSG は client の殻（`public/index.html`、WASM の読込を含む）を待たない。`--force-sequential`（`CI` 環境変数があれば自動）で client → server の順にする。`dx serve`（開発サーバーの下）では SSG の書込をしない。再ビルドのたびに古い HTML が残り、`/` の分が殻として読まれてしまうため
- dx はツール（wasm-bindgen、wasm-opt、Tailwind）を `DX_HOME`（既定 `~/.local/share/.dx`）の `tools/` に落とす。release ビルドは wasm-opt が無いと止まる
