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

# 拾えない形は書けなくする（下の forms）。NG にするのは:
# - 文字に {…} を差し込む class: "… {x}"
# - class: 変数や式、枝が文字でない class: if
# - 省略形の class, と class }（束ねた変数をそのまま置ける）
# 通すのは、文字そのもの、枝が文字の class: if、部品が受けた class をそのまま渡す class: "{class}" だけ。
# 最後の形は、囲む fn の引数に class（&'static str か String）があり、その fn の中で let class と束ね直していないときに限る
# （置く側の class: "…" を下で拾う）。状態の差分は data-state か aria-* の variant で 1 本の文字に書く
# （docs/design/README.md「状態」）。
forms_pl=$(cat <<'PERL'
my $src = $_;
# コメントは同じ長さの空白にする（行と位置を保つ）
$src =~ s{(^|[ \t])(//[^\n]*)}{$1 . (" " x length $2)}gme;
# fn ごとに、始まり・引数の範囲・引数に class があるか
my @fns;
while ($src =~ /\bfn\s+\w+\s*(?:<[^>]*>)?\s*(\((?:[^()]++|(?1))*\))/g) {
  push @fns, [$-[0], $-[1], $+[1], $1 =~ /(?<![\w-])class\s*:\s*(?:&'static\s+str|String)\b/ ? 1 : 0];
}
my $iflit = qr/if\b[^{]*\{\s*"[^"]*"\s*\}(?:\s*else\s*(?:if\b[^{]*)?\{\s*"[^"]*"\s*\})*/;
while ($src =~ /(?<![\w{"-])class\b\s*([:,}])/g) {
  my ($at, $sep, $after) = ($-[0], $1, $+[0]);
  my ($fn) = grep { $_->[0] <= $at } reverse @fns;
  next if $fn && $at > $fn->[1] && $at < $fn->[2];
  my $why;
  if ($sep ne ":") {
    $why = "省略形の class";
  } else {
    my ($v) = substr($src, $after) =~ /^\s*($iflit|"[^"]*"|\S+)/;
    if ($v =~ /^"/) {
      if ($v eq q("{class}")) {
        my $body = substr($src, $fn ? $fn->[0] : 0, $at - ($fn ? $fn->[0] : 0));
        $why = "部品の引数でない class を渡している" unless $fn && $fn->[3] && $body !~ /\blet\s+(?:mut\s+)?class\b/;
      } elsif ($v =~ /[{}]/) {
        $why = "文字に {…} を差し込んでいる";
      }
    } elsif ($v =~ /^if\b/ && $v =~ /\}\s*$/) {
      $why = "枝の文字に {…} を差し込んでいる" if grep { /[{}]/ } $v =~ /"([^"]*)"/g;
    } elsif ($v !~ /^(?:&|String\b|Option\b)/) {
      $why = "class: に変数か式";
    }
  }
  next unless $why;
  my $line = 1 + (substr($src, 0, $at) =~ tr/\n//);
  print "$ARGV:$line: $why\n";
}
PERL
)
forms() { perl -0777 -ne "$forms_pl" $(find "$1" -name '*.rs'); }

# 検査そのものの検査: NG にすべき形が NG になり、通すべき形が通るか
tmp=$(mktemp -d "${TMPDIR:-/tmp}/check-classes.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/bad" "$tmp/good"
cat > "$tmp/bad/interpolated.rs" <<'RS'
fn A(x: &'static str) -> Element { rsx! { div { class: "p-1 {x}" } } }
RS
cat > "$tmp/bad/variable.rs" <<'RS'
fn A(open: bool) -> Element { let c = if open { "a" } else { "b" }; rsx! { div { class: c } } }
RS
cat > "$tmp/bad/if_expr.rs" <<'RS'
fn A(a: bool, b: String) -> Element { rsx! { div { class: if a { b } else { "c".into() } } } }
RS
cat > "$tmp/bad/shorthand_comma.rs" <<'RS'
fn A(class: &'static str) -> Element { rsx! { span { class, "x" } } }
RS
cat > "$tmp/bad/shorthand_brace.rs" <<'RS'
fn A(class: &'static str) -> Element { rsx! { span { class } } }
RS
cat > "$tmp/bad/local_forwarded.rs" <<'RS'
fn A(open: bool) -> Element {
    let class = if open { "a" } else { "b" };
    rsx! { div { class: "{class}" } }
}
RS
cat > "$tmp/bad/rebound_forwarded.rs" <<'RS'
fn A(class: &'static str, open: bool) -> Element {
    let class = if open { "a" } else { class };
    rsx! { div { class: "{class}" } }
}
RS
cat > "$tmp/good/ok.rs" <<'RS'
// class, や class: x はコメントの中なら数えない
fn A(class: &'static str, numeric: bool) -> Element {
    rsx! {
        div { class: "p-1", class: "{class}" }
        input { class: "p-1", class: if numeric { "font-mono" } else { "text-body" } }
        span { subclass: x, my_class: y }
    }
}
RS
out=$(forms "$tmp/bad")
for f in "$tmp"/bad/*.rs; do
  printf '%s\n' "$out" | grep -qF "$f:" || { echo "NG: 検査が $(basename "$f") の形を見逃した"; fail=1; }
done
out=$(forms "$tmp/good")
[ -z "$out" ] || { echo "NG: 検査が通すべき形を落とした: $out"; fail=1; }

forms crates/rustruct-web/src > "$tmp/found"
while IFS= read -r b; do
  echo "NG: $b（状態は data-state か aria-* の variant で 1 本の文字に書く）"; fail=1
done < "$tmp/found"

# class: "…" と、class: if … { "…" } [else …]（行をまたいでもよい）の文字をすべて拾う
classes=$(perl -0777 -ne 'while (/(?<![\w-])class\s*:\s*(if\b[^{]*\{\s*"[^"]*"\s*\}(?:\s*else\s*(?:if\b[^{]*)?\{\s*"[^"]*"\s*\})*|"[^"{}]*")/g) { my $m = $1; print "$1\n" while $m =~ /"([^"]*)"/g }' \
  $(find crates/rustruct-web/src -name '*.rs') | tr ' ' '\n' | grep -v '^$' | sort -u)

for c in $classes; do
  # セレクタでは : . / を \ で逃がす（.sm\:grid-cols-2 の形）
  sel=$(printf '%s' "$c" | sed -E 's#([:./])#\\\1#g')
  if ! grep -qF ".$sel {" "$css"; then
    echo "NG: $c が $css に無い（dx serve か dx build で生成し直して commit する）"; fail=1
  fi
done

# 切り替わる幅は md と lg だけ（ADR site-0007）。Tailwind 既定の sm・xl・2xl（40rem・80rem・96rem）の
# メディアクエリが生成物にあれば、theme.css の --breakpoint-* の消し忘れか、既定の幅が漏れている
if grep -qE '\(width >= (40|80|96)rem\)' "$css"; then
  echo "NG: $css に Tailwind 既定の切り替わる幅（sm / xl / 2xl）がある"; fail=1
fi

[ "$fail" -eq 0 ] && echo "OK: マークアップのクラスはすべて $css にある"
exit "$fail"
