#!/usr/bin/env sh
# dx serve の画面を幅ごとにライト・ダークで撮る。エージェントが見た目を自分で確かめるための道具。
#   sh scripts/screenshot.sh [URL] [出力ディレクトリ] [幅…]
#   既定: http://localhost:8080  $TMPDIR（無ければ /tmp）  375 820 1280（スマホ・iPad・PC。docs/adr/0007）
#   高さは SCREENSHOT_HEIGHT（既定 800）。ページの下のほうを見るときに伸ばす
#   hydration の後に待つ時間は SCREENSHOT_WAIT（ミリ秒、既定 500）
# 出力: <dir>/<幅>-light.png と <dir>/<幅>-dark.png
# 撮るのは scripts/screenshot.ps1（PowerShell）。headless Chrome を DevTools のプロトコルで動かし、幅を Emulation で
# 決め、WASM の起動（hydration）を待ってから撮る。仕組みと、iframe に入れて撮る形をやめた理由はそちらに書いた。
# Chrome / Edge / Chromium を順に探す。WSL では Windows 側の Chrome と PowerShell を使う（Windows の Chrome から
# WSL の localhost には届くが、WSL から Windows の DevTools のポートには届かないので、操る側も Windows に置く）。
# サンドボックスの中からは起動できないので、エージェントはサンドボックスを外して実行する。
set -eu
url=${1:-http://localhost:8080}
dir=${2:-${TMPDIR:-/tmp}}
if [ "$#" -gt 2 ]; then shift 2; else set -- 375 820 1280; fi
height=${SCREENSHOT_HEIGHT:-800}
wait=${SCREENSHOT_WAIT:-500}
mkdir -p "$dir"
dir=$(cd "$dir" && pwd)
here=$(cd "$(dirname "$0")" && pwd)

# dev server が無いと何も撮れないので、先に確かめる（このスクリプトはサンドボックスの外で動く）
curl -sf -m 5 -o /dev/null "$url" || { echo "$url に届かない。dx serve が動いているか見る" >&2; exit 1; }

browser=
for c in chromium chromium-browser google-chrome google-chrome-stable \
  "/mnt/c/Program Files/Google/Chrome/Application/chrome.exe" \
  "/mnt/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"; do
  if command -v "$c" >/dev/null 2>&1 || [ -x "$c" ]; then browser=$c; break; fi
done
[ -n "$browser" ] || { echo "Chrome / Edge / Chromium が見つからない" >&2; exit 1; }

# 操る側の PowerShell。Windows 側のブラウザなら Windows 側の PowerShell
shell=
case "$browser" in
  /mnt/c/*)
    for c in "/mnt/c/Program Files/PowerShell/7/pwsh.exe" \
      "/mnt/c/WINDOWS/System32/WindowsPowerShell/v1.0/powershell.exe"; do
      if [ -x "$c" ]; then shell=$c; break; fi
    done ;;
  *) command -v pwsh >/dev/null 2>&1 && shell=pwsh ;;
esac
[ -n "$shell" ] || { echo "PowerShell が見つからない" >&2; exit 1; }

# Windows 側には Windows のパスで渡す
win() {
  case "$browser" in /mnt/c/*) wslpath -w "$1" ;; *) printf '%s' "$1" ;; esac
}
case "$browser" in /mnt/c/*) browser_path=$(wslpath -w "$browser") ;; *) browser_path=$(command -v "$browser") ;; esac

json() { printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'; }
widths=$(printf '%s,' "$@")
cfg="$dir/screenshot.json"
printf '{"browser": "%s", "url": "%s", "dir": "%s", "height": %s, "widths": [%s], "wait": %s}\n' \
  "$(json "$browser_path")" "$(json "$url")" "$(json "$(win "$dir")")" "$height" "${widths%,}" "$wait" > "$cfg"

# WSL のファイル（\\wsl.localhost\...）は Windows からはネットワーク上のスクリプトに見え、実行ポリシー（RemoteSigned）が
# -File での実行を止める。リポジトリの中のスクリプトなので、中身を読んで scriptblock として実行する
ps1=$(win "$here/screenshot.ps1")
"$shell" -NoProfile -NonInteractive -Command "& ([scriptblock]::Create([IO.File]::ReadAllText('$ps1'))) '$(win "$cfg")'"
rm -f "$cfg"
for width in "$@"; do
  echo "$dir/$width-light.png"
  echo "$dir/$width-dark.png"
done
