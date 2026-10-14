#!/usr/bin/env sh
# SSG 出力の煙テスト（Issue #12）。`dx build --ssg --release --force-sequential` が出した配信物を外から見る。
#   sh scripts/check-ssg.sh [出力ディレクトリ]
#   既定: target/dx/rustruct-web/release/web/public
# 見るもの: 住所ごとの HTML が静的なファイルとしてあり（SPA のフォールバックに頼らない）、JavaScript 無しで
# 読める本文（wordmark、一覧、見出し、<title>、description）を含み、CSS と WASM が 1 つずつあり、
# HTML が CSS と読込スクリプトを、スクリプトが WASM を参照していること。文言の正しさは tests/render.rs が
# 見るので、ここは存在と参照だけ。hydration の成否はブラウザが要るので見ない。
set -eu
cd "$(dirname "$0")/.."
out=${1:-target/dx/rustruct-web/release/web/public}
site=rustruct.matsumok.com
fail=0
ng() { echo "NG: $1"; fail=1; }

[ -d "$out" ] || { echo "NG: $out が無い（先に dx build --ssg --release --force-sequential）"; exit 1; }

# 住所の集合。crates/rustruct-web/src/ssg.rs の static_paths と同じ（tests/render.rs も同じ一覧を期待値に持つ）
expected="/ /tools /studies /notes /root /404"
sections="/tools /studies /notes"
found=$(cd "$out" && find . -name index.html | sed 's#^\./##; s#/\{0,1\}index\.html$##; s#^#/#' | sort)
if [ "$found" != "$(printf '%s\n' $expected | sort)" ]; then
  ng "住所ごとの HTML が期待と違う。期待: $expected / 実際: $(printf '%s ' $found)"
fi

# CSS と WASM は 1 つずつ
one() { # $1=名前 $2=拡張子 → 見つかったファイル（1 つでなければ NG で空）
  files=$(find "$out" -name "*.$2")
  if [ "$(printf '%s\n' "$files" | grep -c .)" -eq 1 ]; then printf '%s' "$files"; else ng "$1 が 1 つでない: ${files:-（無し）}"; fi
}
css=$(one CSS css)
wasm=$(one WASM wasm)

for path in $expected; do
  html="$out${path%/}/index.html"
  [ -f "$html" ] || continue # 集合の検査で既に NG
  grep -qF " · $site</title>" "$html" || ng "$path の <title> が「ページ名 · $site」でない"
  grep -qF '<meta name="description" content="' "$html" || ng "$path に description が無い"
  grep -qF '>rustruct<' "$html" || ng "$path に wordmark が無い"
  grep -qF '>.matsumok.com<' "$html" || ng "$path の wordmark が分かれていない"
  [ "$path" = / ] || grep -qF '<h1' "$html" || ng "$path に見出し（h1）が無い"
  [ -z "$css" ] || grep -qF "$(basename "$css")" "$html" || ng "$path が CSS $(basename "$css") を参照していない"
  # HTML → 読込スクリプト → WASM
  js=$(sed -n 's/.*<script type="module" async src="\([^"]*\)".*/\1/p' "$html" | head -n 1)
  if [ -z "$js" ]; then
    ng "$path の HTML に WASM の読込スクリプトが無い"
  elif [ ! -f "$out/${js#/}" ]; then
    ng "$path が参照するスクリプトが無い: $js"
  elif [ -n "$wasm" ] && ! grep -qF "$(basename "$wasm")" "$out/${js#/}"; then
    ng "$path のスクリプト $js が WASM $(basename "$wasm") を参照していない"
  fi
done

# トップの一覧は三部へのリンク
for s in $sections; do
  grep -qF "href=\"$s\"" "$out/index.html" || ng "/ の一覧に $s へのリンクが無い"
done

# not-found は空ページの文言と混ざらない
grep -qF 'この住所には何もない。' "$out/404/index.html" || ng "/404 に「この住所には何もない。」が無い"
grep -qF 'まだ何もない' "$out/404/index.html" && ng "/404 が空ページの文言になっている"

[ "$fail" -eq 0 ] && echo "OK: SSG の出力 $out は住所ごとの HTML と CSS・WASM を揃えている"
exit "$fail"
