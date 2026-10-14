#!/usr/bin/env sh
# dx serve の画面をスマホ幅でライト・ダーク 2 枚撮る。エージェントが見た目を自分で確かめるための道具。
#   sh scripts/screenshot.sh [URL] [出力ディレクトリ]
#   既定: http://localhost:8080  $TMPDIR（無ければ /tmp）
# 出力: <dir>/light.png と <dir>/dark.png（スマホ幅 400x800）
# Chrome / Edge / Chromium を順に探す。WSL では Windows 側の Chrome を使う（localhost は WSL に届く。
# サンドボックスの中からは起動できないので、エージェントはサンドボックスを外して実行する）。
# headless Chrome はウィンドウ幅を 500px より狭くできず、--window-size=400 でも 500px の画面を
# 400px に切り抜くだけになる。それでは折り返しとはみ出しを誤診するので、スマホ幅の iframe に
# ページを入れた <dir>/frame.html を撮る。
set -eu
url=${1:-http://localhost:8080}
dir=${2:-${TMPDIR:-/tmp}}
width=400
height=800
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

cat > "$dir/frame.html" <<EOF
<!doctype html><html><head><meta charset="utf-8">
<style>html,body{margin:0}iframe{display:block;border:0;width:${width}px;height:${height}px}</style>
</head><body><iframe src="$url"></iframe></body></html>
EOF

shoot() { # $1=ファイル名 $2=追加フラグ
  "$browser" --headless=new --disable-gpu --hide-scrollbars --no-first-run \
    --user-data-dir="$(out "$dir/profile")" \
    --window-size=$width,$height --virtual-time-budget=5000 $2 \
    --screenshot="$(out "$dir/$1")" "$(file_url "$dir/frame.html")" >/dev/null 2>&1
  echo "$dir/$1"
}

shoot light.png ""
shoot dark.png "--force-dark-mode"
