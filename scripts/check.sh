#!/usr/bin/env sh
# CI（.github/workflows/ci.yml）と手元で同じ検査を回す。コマンドの定義はここだけ。
#   sh scripts/check.sh          すべてを CI と同じ順で
#   sh scripts/check.sh clippy   一つだけ（名前は下の case のとおり）
set -eu
cd "$(dirname "$0")/.."

step() {
  case "$1" in
    fmt)     cargo fmt --all --check ;;
    deps)    sh scripts/check-deps.sh ;;
    markup)  sh scripts/check-markup.sh ;;
    classes) sh scripts/check-classes.sh ;;
    clippy)  cargo clippy --workspace --all-targets --locked -- -D warnings ;;
    server)  cargo clippy -p rustruct-web --features server --locked -- -D warnings ;;
    wasm)    cargo check -p rustruct-web --features web --target wasm32-unknown-unknown --locked ;;
    test)    cargo test --workspace --locked ;;
    # build.rs が tokens.json から theme.css を作り直す。差分が出たら生成物の commit 忘れ
    theme)   git diff --exit-code -- crates/rustruct-web/theme.css ;;
    # SSG（ADR-0001）。client → server の順に建てる。並行だと server の SSG が client の index.html（殻）
    # を待たずに走り、WASM を読まない HTML になる。出力の検査は check-ssg.sh
    ssg)     dx build --ssg --release --force-sequential && sh scripts/check-ssg.sh ;;
    # dx build が内蔵 Tailwind で assets/tailwind.css を作り直す。差分が出たら生成物の commit 忘れ。
    # dx は Tailwind の失敗を黙殺するので、走らなかったときは差分ゼロで素通りする。classes が補う
    tailwind) git diff --exit-code -- crates/rustruct-web/assets/tailwind.css ;;
    *) echo "unknown step: $1" >&2; exit 2 ;;
  esac
}

if [ "$#" -gt 0 ]; then
  for s in "$@"; do step "$s"; done
else
  for s in fmt deps markup classes clippy server wasm test theme ssg tailwind; do
    echo "== $s"; step "$s"
  done
fi
