#!/usr/bin/env sh
# SSG の出力のスモークテスト（Issue #12）。`dx build --ssg --release --force-sequential` が出した配信物を外から見る。
#   sh scripts/check-ssg.sh [出力ディレクトリ]
#   既定: target/dx/rustruct-web/release/web/public
# 見るものと読み方は crates/rustruct-web/src/ssg.rs（example check-ssg が呼ぶ）。
exec cargo run -q --manifest-path "$(dirname "$0")/../Cargo.toml" -p rustruct-web --example check-ssg --locked -- "$@"
