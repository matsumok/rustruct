#!/usr/bin/env sh
# SSG の出力を headless Chrome で開き、hydration が成功したかを DOM とコンソールから見る。
#   sh scripts/hydrate-check.sh [URL か出力ディレクトリ] [住所...]
#   既定: target/dx/rustruct-web/release/web/public を 127.0.0.1:8090 で配る。住所は / /tools/ /404/
#   例: sh scripts/hydrate-check.sh https://rustruct.matsumok.com / /studies/
# 見るもの（住所ごと）:
#   - 起動後の DOM に data-dioxus-id（hydration がリスナーを付けた印）があり、data-node-hydration（プリレンダー時の
#     印）が残っている。残っていれば既存の要素が再利用された。hydration が失敗すると要素が作り直されて印が消える
#   - ページのコンソール出力（Chrome の stderr の CONSOLE 行）が無い
# 「項目を押して再読み込みが起きない」「戻るが効く」は DevTools プロトコルが要るので見ない。
# Chrome / Edge / Chromium は screenshot.sh と同じ探し方。WSL では Windows 側を使う。
# サンドボックスの外で動かす（ブラウザがサンドボックス内のサーバーに届かない）。
set -u
cd "$(dirname "$0")/.."
target=${1:-target/dx/rustruct-web/release/web/public}
[ "$#" -gt 0 ] && shift
[ "$#" -gt 0 ] || set -- / /tools/ /404/
dir=${TMPDIR:-/tmp}/hydrate-check
rm -rf "$dir"; mkdir -p "$dir"
fail=0

browser=
for c in chromium chromium-browser google-chrome google-chrome-stable \
  "/mnt/c/Program Files/Google/Chrome/Application/chrome.exe" \
  "/mnt/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"; do
  if command -v "$c" >/dev/null 2>&1 || [ -x "$c" ]; then browser=$c; break; fi
done
[ -n "$browser" ] || { echo "Chrome / Edge / Chromium が見つからない" >&2; exit 1; }
out() { case "$browser" in /mnt/c/*) wslpath -w "$1" ;; *) printf '%s' "$1" ;; esac; }

# 出力ディレクトリなら自分で配る
srv=
case "$target" in
  http://*|https://*) url=${target%/} ;;
  *)
    [ -d "$target" ] || { echo "$target が無い（先に dx build --ssg --release --force-sequential）" >&2; exit 1; }
    python3 -m http.server 8090 --bind 127.0.0.1 --directory "$target" >"$dir/server.log" 2>&1 &
    srv=$!
    sleep 1
    url=http://127.0.0.1:8090
    ;;
esac

for p in "$@"; do
  name=$(printf '%s' "$p" | tr '/' '_')
  "$browser" --headless=new --disable-gpu --no-first-run --hide-scrollbars \
    --user-data-dir="$(out "$dir/profile")" \
    --virtual-time-budget=15000 --enable-logging=stderr --v=0 \
    --dump-dom "$url$p" >"$dir/dom$name.html" 2>"$dir/log$name.txt"
  hydrated=$(grep -o 'data-dioxus-id="[0-9]*"' "$dir/dom$name.html" | wc -l)
  kept=$(grep -o 'data-node-hydration=' "$dir/dom$name.html" | wc -l)
  console=$(grep 'CONSOLE(' "$dir/log$name.txt" | cut -c1-200)
  echo "== $url$p"
  echo "   リスナーの付いた要素: $hydrated  プリレンダーの印が残る要素: $kept"
  if [ "$hydrated" -eq 0 ]; then echo "   NG: hydration の印が無い（WASM が動いていないか失敗した）"; fail=1; fi
  if [ "$hydrated" -gt 0 ] && [ "$kept" -eq 0 ]; then echo "   NG: プリレンダーの印が消えた（hydration が失敗して作り直した）"; fail=1; fi
  if [ -n "$console" ]; then echo "   NG: コンソール出力がある:"; echo "$console" | sed 's/^/     /'; fail=1; fi
done

[ -n "$srv" ] && kill "$srv"
[ "$fail" -eq 0 ] && echo "OK: hydration は成功（DOM と log は $dir）"
exit "$fail"
