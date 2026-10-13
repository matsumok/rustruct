#!/usr/bin/env sh
# マークアップに値を書かない検査（docs/design/README.md「マークアップに色の値を直接書かない」）。
# 画面のコードには役割名のクラスだけを書く。値は docs/design/tokens.json にだけある。
# src/theme.rs は tokens.json の生成器とそのテストなので、値を持つのが仕事。対象外。
set -eu
cd "$(dirname "$0")/.."
fail=0

check() { # $1=何を見つけたか $2=正規表現
  if hits=$(grep -rnE --include="*.rs" --exclude=theme.rs -e "$2" crates/rustruct-web/src); then
    echo "NG: $1"; echo "$hits"; fail=1
  fi
}

check "色の値（#rrggbb / #rrggbbaa）"       '#([0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b'
check "長さの値（px / rem / em）"            '\b[0-9]+(\.[0-9]+)?(px|rem|em)\b'
check "Tailwind の任意値 [...]"              '-\[[^]]+\]'
check "Tailwind 既定のパレット名"            '\b(bg|text|border|fill|stroke|ring|outline|decoration|accent|caret|shadow)-(red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|slate|gray|zinc|neutral|stone|white|black)(-[0-9]+)?\b'
check "style 属性（CSS の直書き）"           '\bstyle: '

[ "$fail" -eq 0 ] && echo "OK: マークアップに値は無い"
exit "$fail"
