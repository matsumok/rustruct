#!/usr/bin/env sh
# 生成物 assets/tailwind.css がマークアップと合っている検査。
# assets/tailwind.css は dx（serve / build）が内蔵 Tailwind でマークアップから作る。dx が動いていない
# 環境でクラスを足すと生成物が古いまま commit され、dx serve 無しの確認では見た目に出ない。
# ここでは src の class: "..." に書かれた各クラスが生成物にセレクタとして存在するかを見る。
# Tailwind CLI は要らない。CI では dx build 後に check.sh の tailwind（git diff）も回るが、dx は Tailwind の
# 失敗を黙殺して差分ゼロで通すことがあるので、この検査は残す。
set -eu
cd "$(dirname "$0")/.."
css=crates/rustruct-web/assets/tailwind.css
fail=0

classes=$(grep -rhoE 'class: "[^"{}]*"' crates/rustruct-web/src \
  | sed -E 's/^class: "//; s/"$//' | tr ' ' '\n' | grep -v '^$' | sort -u)

for c in $classes; do
  # セレクタでは : . / を \ で逃がす（.sm\:grid-cols-2 の形）
  sel=$(printf '%s' "$c" | sed -E 's#([:./])#\\\1#g')
  if ! grep -qF ".$sel {" "$css"; then
    echo "NG: $c が $css に無い（dx serve か dx build で生成し直して commit する）"; fail=1
  fi
done

[ "$fail" -eq 0 ] && echo "OK: マークアップのクラスはすべて $css にある"
exit "$fail"
