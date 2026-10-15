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

# class: "…" と、class: if … { "…" } else { "…" }（行をまたいでもよい）の文字をすべて拾う
classes=$(perl -0777 -ne 'while (/class:\s*(if\b[^{]*\{\s*"[^"]*"\s*\}\s*else\s*\{\s*"[^"]*"\s*\}|"[^"{}]*")/g) { my $m = $1; print "$1\n" while $m =~ /"([^"]*)"/g }' \
  $(find crates/rustruct-web/src -name '*.rs') | tr ' ' '\n' | grep -v '^$' | sort -u)

for c in $classes; do
  # セレクタでは : . / を \ で逃がす（.sm\:grid-cols-2 の形）
  sel=$(printf '%s' "$c" | sed -E 's#([:./])#\\\1#g')
  if ! grep -qF ".$sel {" "$css"; then
    echo "NG: $c が $css に無い（dx serve か dx build で生成し直して commit する）"; fail=1
  fi
done

# 切り替わる幅は md と lg だけ（docs/adr/0007）。Tailwind 既定の sm・xl・2xl（40rem・80rem・96rem）の
# メディアクエリが生成物にあれば、theme.css の --breakpoint-* の消し忘れか、既定の幅が漏れている
if grep -qE '\(width >= (40|80|96)rem\)' "$css"; then
  echo "NG: $css に Tailwind 既定の切り替わる幅（sm / xl / 2xl）がある"; fail=1
fi

[ "$fail" -eq 0 ] && echo "OK: マークアップのクラスはすべて $css にある"
exit "$fail"
