#!/usr/bin/env sh
# SSG の出力を配信用に組み替える（Issue #13）。wrangler.toml の assets.directory はこの出力を指す。
#   sh scripts/stage-deploy.sh [SSG の出力] [配信用ディレクトリ]
#   既定: target/dx/rustruct-web/release/web/public → target/deploy
# 変えるのは 1 点だけ。SSG は not-found を 404/index.html に書く（dioxus-server は住所の下に必ず
# index.html を置く。docs/dioxus-0.7.md）が、Workers Static Assets が未知の住所に返すのはルートの
# 404.html（not_found_handling = "404-page"）。そこへ移し、404/ は残さない（同じ中身の住所を 2 つ作らない）。
# 検査は crates/rustruct-web/tests/deploy.rs
set -eu
cd "$(dirname "$0")/.."
ssg_out=${1:-target/dx/rustruct-web/release/web/public}
deploy=${2:-target/deploy}

[ -d "$ssg_out" ] || { echo "$ssg_out が無い（先に dx build --ssg --release --force-sequential）" >&2; exit 1; }
[ -f "$ssg_out/404/index.html" ] || { echo "$ssg_out/404/index.html が無い（SSG が not-found を書いていない）" >&2; exit 1; }

rm -rf "$deploy"
mkdir -p "$deploy"
cp -R "$ssg_out/." "$deploy"
mv "$deploy/404/index.html" "$deploy/404.html"
rmdir "$deploy/404"
echo "配信物: $deploy（not-found は 404.html）"
