# rustruct

構造設計者向けの計算ツールと、数学的な可視化・書きものを置く個人サイト（rustruct.matsumok.com）。
思想と経緯は `docs/philosophy.md`、UI の決め事は `docs/ui-decisions.md`、骨格と技術スタックは `docs/stack.md`、Dioxus 0.7 でコードから読めない挙動は `docs/dioxus-0.7.md`（無いことは vendored ソースを grep する）を参照。

## Commands

Rust は stable。`rust-toolchain.toml` が channel と wasm ターゲットを指定する。`dx` は Dioxus CLI 0.7 系（`cargo install dioxus-cli --version 0.7.10 --locked`、または GitHub Releases のビルド済みバイナリ）。

- 起動: `dx serve` をリポジトリ直下で。`default-members` が `rustruct-web` を指すので `-p` は不要。`web` / `server` の feature を dx が自動検出して fullstack で起動する
- テスト: `cargo test --workspace`。feature なしのネイティブで、WASM もブラウザも使わない
- 検査: `sh scripts/check.sh` が CI（`.github/workflows/ci.yml`）と同じものを同じ順で回す。一つだけなら `sh scripts/check.sh clippy` のように名前で。`ssg` は `dx build --ssg --release --force-sequential` を回すので時間がかかる（出力は `target/dx/rustruct-web/release/web/public`）。手で打つときも `--force-sequential` を付ける。無いと client と server が並行に建ち、SSG が WASM を読まない殻で描くことがある（`docs/stack.md`）
- 見た目の確認: `dx serve` 中に `sh scripts/screenshot.sh` でスマホ幅のライト・ダークを撮れる。hydration の成否は `sh scripts/hydrate-check.sh`（SSG の出力か URL を headless Chrome で開き、DOM の印とコンソールを見る）。`dx serve` もこのスクリプトもサンドボックスの外で起動する（中からはブラウザにもホストにも届かず、中の `curl localhost` も失敗する）。`.claude/settings.local.json` の `excludedCommands` に挙げた `dx serve`、`sh scripts/screenshot.sh`、`sh scripts/hydrate-check.sh` は外で走るが、パターンは前方一致。`cd` や `&&` を付けず、パターンどおりの形で単独のコマンドとして呼ぶ（外れるとサンドボックス内に戻り、黙って失敗する）。dev server の生死はスクリプトが先に確かめて、無ければ失敗して言う。前のセッションの `dx serve` が残っていたら `ps -eo pid,etimes,comm | grep -E 'dx$|tailwindcss|server-'` で見て止める
- dx はツール（Tailwind、wasm-bindgen、wasm-opt）を `~/.local/share/.dx/tools`（`DX_HOME` で変えられる）に落とす。Tailwind は落とせなくても黙って続行するので、`assets/tailwind.css` が更新されないときはそこを見る。wasm-opt（release ビルド）は落とせないと "Failed to create wasm-opt instance" で止まる
- レビューの規則は `CODING_STANDARDS.md`

## Agent skills

### Issue tracker

Issues live in GitHub Issues for `matsumok/rustruct` (via the `gh` CLI). See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles use their default label strings (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
