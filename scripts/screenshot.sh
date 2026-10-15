#!/usr/bin/env sh
# dx serve の画面を幅ごとにライト・ダークで撮る。エージェントが見た目を自分で確かめるための道具。
#   sh scripts/screenshot.sh [URL] [出力ディレクトリ] [幅…]
#   既定: http://localhost:8080  $TMPDIR（無ければ /tmp）  375 820 1280（スマホ・iPad・PC。docs/adr/0007）
#   高さは SCREENSHOT_HEIGHT（既定 800）。ページの下のほうを見るときに伸ばす
# 出力: <dir>/<幅>-light.png と <dir>/<幅>-dark.png
# ダークは OS 設定（--force-dark-mode）で撮る。ページで選んだ色はプロファイルごと毎回消すので残らない
# Chrome / Edge / Chromium を順に探す。WSL では Windows 側の Chrome を使う（localhost は WSL に届く。
# サンドボックスの中からは起動できないので、エージェントはサンドボックスを外して実行する）。
# headless Chrome はウィンドウ幅を 500px より狭くできず、--window-size=400 でも 500px の画面を
# 400px に切り抜くだけになる。それでは折り返しとはみ出しを誤診するので、スマホ幅の iframe に
# ページを入れた <dir>/frame.html を撮る。
set -eu
url=${1:-http://localhost:8080}
dir=${2:-${TMPDIR:-/tmp}}
if [ "$#" -gt 2 ]; then shift 2; else set -- 375 820 1280; fi
height=${SCREENSHOT_HEIGHT:-800}
mkdir -p "$dir"

# dev server が無いと Chrome は何も書かずに終わるので、先に確かめる（このスクリプトはサンドボックスの外で動く）
curl -sf -m 5 -o /dev/null "$url" || { echo "$url に届かない。dx serve が動いているか見る" >&2; exit 1; }

browser=
for c in chromium chromium-browser google-chrome google-chrome-stable \
  "/mnt/c/Program Files/Google/Chrome/Application/chrome.exe" \
  "/mnt/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"; do
  if command -v "$c" >/dev/null 2>&1 || [ -x "$c" ]; then browser=$c; break; fi
done
[ -n "$browser" ] || { echo "Chrome / Edge / Chromium が見つからない" >&2; exit 1; }

# Windows 側のブラウザには Windows のパスで渡す
out() {
  case "$browser" in /mnt/c/*) wslpath -w "$1" ;; *) printf '%s' "$1" ;; esac
}

# ローカルファイルの URL。UNC（\\wsl.localhost\...）は file://host/..、ドライブは file:///C:/..
file_url() {
  p=$(out "$1" | tr '\\' '/')
  case "$p" in //*) printf 'file:%s' "$p" ;; *) printf 'file:///%s' "$p" ;; esac
}

shoot() { # $1=幅 $2=ファイル名 $3=追加フラグ
  cat > "$dir/frame.html" <<EOF
<!doctype html><html><head><meta charset="utf-8">
<style>html,body{margin:0}iframe{display:block;border:0;width:${1}px;height:${height}px}</style>
</head><body><iframe src="$url"></iframe></body></html>
EOF
  rm -rf "$dir/profile"
  "$browser" --headless=new --disable-gpu --hide-scrollbars --no-first-run \
    --user-data-dir="$(out "$dir/profile")" \
    --window-size=$1,$height --virtual-time-budget=5000 $3 \
    --screenshot="$(out "$dir/$2")" "$(file_url "$dir/frame.html")" >/dev/null 2>&1
  echo "$dir/$2"
}

for width in "$@"; do
  shoot "$width" "$width-light.png" ""
  shoot "$width" "$width-dark.png" "--force-dark-mode"
done
