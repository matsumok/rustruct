#!/usr/bin/env sh
# 配置後のスモークテスト（Issue #13）。本番やプレビューの URL を外から見る。CI が配置の直後に回す。
#   sh scripts/check-live.sh URL
#   例: sh scripts/check-live.sh https://rustruct.matsumok.com
# 見るもの: 住所ごとの HTML が 200 で届き、末尾スラッシュ無しは 307 で揃えられ、未知の住所は 404 で not-found の
# 本文が返り、HTML が参照する CSS・読込スクリプト・WASM が届き、WASM の Content-Type が application/wasm
# （ストリーミング compile の条件）。フォントの link とダーク追従の CSS も、配信物に入っていることだけ見る。
# hydration の成否はブラウザが要るので見ない（scripts/hydrate-check.sh に URL を渡す）。
# set -e は付けない。curl が失敗しても status 000 の NG として続け、落ちている点を全部並べる。
set -u
url=${1:?URL を渡す（例: https://rustruct.matsumok.com）}
url=${url%/}
dir=${TMPDIR:-/tmp}/check-live
rm -rf "$dir"; mkdir -p "$dir"
fail=0
ng() { echo "NG: $1"; fail=1; }

# fetch 住所 保存先 → 標準出力に status、Location は $dir/location に
fetch() {
  curl -sS -o "$2" -D "$dir/headers" -w '%{http_code}' "$url$1"
  sed -n 's/^[Ll]ocation: *//p' "$dir/headers" | tr -d '\r' >"$dir/location"
}
# 直前の fetch が弾かれた理由の手がかり。Cloudflare の防御（Bot Fight Mode、WAF）は cf-mitigated を付ける。
# GitHub の runner からの curl が 403 になったとき（2026-10-14）、これが無いと何に弾かれたか分からなかった
why() {
  m=$(sed -n 's/^[Cc][Ff]-[Mm]itigated: *//p' "$dir/headers" | tr -d '\r')
  [ -n "$m" ] && printf ' (cf-mitigated: %s)' "$m"
  [ -s "$dir/page.html" ] && printf ' 本文: %s' "$(tr -d '\n' <"$dir/page.html" | head -c 120)"
  return 0
}

# 配置の直後は DNS と証明書がまだ届かず、接続できない（status 000）ことがある。初回の本番配置で実際に起きた。
# 応答が返るまで最大 3 分待ってから見る（何かが返れば中身の検査に進む。中身の NG は待っても変わらない）
waited=0
while [ "$(fetch / "$dir/probe.html" 2>/dev/null)" = 000 ] && [ "$waited" -lt 180 ]; do
  [ "$waited" -eq 0 ] && echo "$url に接続できない。DNS・証明書の反映を待つ（最大 3 分）"
  sleep 10; waited=$((waited + 10))
done
[ "$waited" -gt 0 ] && echo "   $waited 秒待った"

# 住所ごとの HTML。一覧は crates/rustruct-web/src/ssg.rs の static_paths と同じ（check-ssg.sh と同様、not-found は別に見る）。
# /root/ に Cloudflare Access を掛けたら（docs/stack.md）ここは 200 でなくなるので、そのときこの一覧から外す
for p in / /tools/ /studies/ /notes/ /root/; do
  status=$(fetch "$p" "$dir/page.html")
  [ "$status" = 200 ] || ng "$p が 200 でない: $status$(why)"
  grep -qF '>rustruct<' "$dir/page.html" || ng "$p に wordmark が無い"
  [ "$p" = / ] || grep -qF '<h1' "$dir/page.html" || ng "$p に見出し（h1）が無い"
done

# 末尾スラッシュ無しは揃えられる（html_handling = "auto-trailing-slash"。文書では 307。308 も転送として受ける）
status=$(fetch /tools "$dir/redirect.html")
case "$status" in
  307|308) grep -q '/tools/$' "$dir/location" || ng "/tools の転送先が /tools/ でない: $(cat "$dir/location")" ;;
  *) ng "/tools が転送されない: $status" ;;
esac

# 未知の住所は 404 で not-found の本文（not_found_handling = "404-page"）。文言は tests/render.rs・check-ssg.sh と同じ。
# workers.dev のプレビューでは wrangler preview が not_found_handling を落とし、本文の無い 404 が返る
# （cloudflare/workers-sdk#16047、2026-10-14 時点で open）。そこだけ NG にせず注意に留める。本番は厳しいまま
status=$(fetch "/no-such-address-$$/" "$dir/notfound.html")
[ "$status" = 404 ] || ng "未知の住所が 404 でない: $status"
if ! grep -qF 'この住所には何もない。' "$dir/notfound.html"; then
  case "$url" in
    *.workers.dev) echo "注意: プレビューでは not-found の本文が届かない（workers-sdk#16047 が直るまで）" ;;
    *) ng "未知の住所に not-found の本文が無い" ;;
  esac
fi

# HTML → CSS、読込スクリプト → WASM
fetch / "$dir/index.html" >/dev/null
css=$(sed -n 's/.*<link rel="stylesheet" href="\(\/assets\/[^"]*\.css\)".*/\1/p' "$dir/index.html" | head -n 1)
js=$(sed -n 's/.*<script type="module" async src="\([^"]*\)".*/\1/p' "$dir/index.html" | head -n 1)
[ -n "$css" ] || ng "/ が CSS を参照していない"
[ -n "$js" ] || ng "/ に WASM の読込スクリプトが無い"
if [ -n "$css" ]; then
  status=$(fetch "$css" "$dir/site.css")
  [ "$status" = 200 ] || ng "CSS $css が 200 でない: $status"
  grep -qF 'prefers-color-scheme' "$dir/site.css" || ng "CSS にダーク追従（prefers-color-scheme）が無い"
fi
if [ -n "$js" ]; then
  js=/${js#/}
  status=$(fetch "$js" "$dir/loader.js")
  [ "$status" = 200 ] || ng "読込スクリプト $js が 200 でない: $status"
  # 読込スクリプトは WASM を "/./assets/<名前>-<ハッシュ>.wasm" の形で指す（住所でない素の名前も別に 1 つ現れる）
  wasm=$(grep -o '"/[A-Za-z0-9_./-]*\.wasm"' "$dir/loader.js" | tr -d '"' | head -n 1)
  if [ -z "$wasm" ]; then
    ng "読込スクリプト $js が WASM を住所で参照していない"
  else
    status=$(fetch "/assets/$(basename "$wasm")" "$dir/app.wasm")
    [ "$status" = 200 ] || ng "WASM $wasm が 200 でない: $status"
    grep -qi '^content-type: *application/wasm' "$dir/headers" || ng "WASM の Content-Type が application/wasm でない: $(grep -i '^content-type' "$dir/headers")"
  fi
fi
grep -qF 'fonts.googleapis.com' "$dir/index.html" || ng "/ にフォントの link が無い"

[ "$fail" -eq 0 ] && echo "OK: $url は住所ごとの HTML・not-found・CSS・WASM を配っている（取得物は $dir）"
exit "$fail"
