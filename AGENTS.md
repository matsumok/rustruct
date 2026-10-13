# rustruct

構造設計者向けの計算ツールと、数学的な可視化・書きものを置く個人サイト（rustruct.matsumok.com）。
思想と経緯は `docs/philosophy.md`、UI の決め事は `docs/ui-decisions.md`、骨格と技術スタックは `docs/stack.md` を参照。

## Commands

Rust は stable。`rust-toolchain.toml` が channel と wasm ターゲットを指定する。`dx` は Dioxus CLI 0.7 系（`cargo install dioxus-cli --version 0.7.10 --locked`、または GitHub Releases のビルド済みバイナリ）。

- 起動: `dx serve` をリポジトリ直下で。`default-members` が `rustruct-web` を指すので `-p` は不要。`web` / `server` の feature を dx が自動検出して fullstack で起動する
- テスト: `cargo test --workspace`。feature なしのネイティブで、WASM もブラウザも使わない
- CI（`.github/workflows/ci.yml`）は push ごとに fmt、依存の向き（`scripts/check-deps.sh`）、clippy、wasm32 の check、test を順に回す。手元で同じものを回すときはそのファイルの step をそのまま実行する

## Agent skills

### Issue tracker

Issues live in GitHub Issues for `matsumok/rustruct` (via the `gh` CLI). See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles use their default label strings (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
