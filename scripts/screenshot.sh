#!/usr/bin/env sh
# dx serve の画面をスマホ幅でライト・ダーク 2 枚撮る。エージェントが見た目を自分で確かめるための道具。
#   sh scripts/screenshot.sh [URL] [出力ディレクトリ]
#   既定: http://localhost:8080  $TMPDIR（無ければ /tmp）
# 出力: <dir>/light.png と <dir>/dark.png（400x800）
# Chrome / Edge / Chromium を順に探す。WSL では Windows 側の Chrome を使う（localhost は WSL に届く）。
set -eu
url=${1:-http://localhost:8080}
dir=${2:-${TMPDIR:-/tmp}}
mkdir -p "$dir"

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

shoot() { # $1=ファイル名 $2=追加フラグ
  "$browser" --headless=new --disable-gpu --hide-scrollbars --no-first-run \
    --window-size=400,800 --virtual-time-budget=5000 $2 \
    --screenshot="$(out "$dir/$1")" "$url" >/dev/null 2>&1
  echo "$dir/$1"
}

shoot light.png ""
shoot dark.png "--force-dark-mode"
