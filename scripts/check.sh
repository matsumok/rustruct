#!/usr/bin/env sh
# CI（.github/workflows/ci.yml）と手元で同じ検査を回す。コマンドの定義はここだけ。
#   sh scripts/check.sh          すべてを CI と同じ順で
#   sh scripts/check.sh clippy   一つだけ（名前は下の case のとおり）
#   sh scripts/check.sh live URL [配信用ディレクトリ]   配置した URL のスモークテスト（引数つきなので既定の列の外。
#                                                    配信用ディレクトリの既定は stage の出力 target/deploy）
set -eu
cd "$(dirname "$0")/.."

# 配置後のスモークテスト。CI は配置の job（Rust は入れていない）から check-live.sh を直に呼ぶ
if [ "${1:-}" = live ]; then
  shift
  url=${1:?URL を渡す（例: sh scripts/check.sh live https://rustruct.matsumok.com）}
  exec sh scripts/check-live.sh "$url" "${2:-target/deploy}"
fi

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
    # SSG（ADR site-0001）。client → server の順に建てる。並行だと server の SSG が client の index.html（殻）
    # を待たずに走り、WASM を読まない HTML になる
    # 前のビルドの出力は先に消す。dx は古いハッシュ付きのアセットを残し、smoke が「1 つずつ」で落ちる
    build)   rm -rf target/dx/rustruct-web/release/web && dx build --ssg --release --force-sequential ;;
    # SSG の出力のスモークテスト。CI では build と別の step にして、建たないのか中身が違うのかを分ける
    smoke)   sh scripts/check-ssg.sh ;;
    ssg)     step build && step smoke ;;
    # 配信用ディレクトリ target/deploy（not-found を 404.html に）。CI はこれを配置の job へ渡す
    stage)   sh scripts/stage-deploy.sh ;;
    # dx build が内蔵 Tailwind で assets/tailwind.css を作り直す。差分が出たら生成物の commit 忘れ。
    # dx は Tailwind の失敗を黙殺するので、走らなかったときは差分ゼロで素通りする。classes が補う
    tailwind) git diff --exit-code -- crates/rustruct-web/assets/tailwind.css ;;
    *) echo "unknown step: $1" >&2; exit 2 ;;
  esac
}

if [ "$#" -gt 0 ]; then
  for s in "$@"; do step "$s"; done
else
  for s in fmt deps markup classes clippy server wasm test theme build smoke tailwind stage; do
    echo "== $s"; step "$s"
  done
fi
