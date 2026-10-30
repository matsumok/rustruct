#!/usr/bin/env sh
# 依存の向きの検査。docs/stack.md のクレート表を機械で守る。
# normal 依存だけを見る（dev-dependencies は対象外）。
set -eu
cd "$(dirname "$0")/.."
fail=0

tree() { pkg=$1; shift; cargo tree -p "$pkg" -e normal --prefix none --locked "$@"; }
direct_deps() { tree "$1" --depth 1 | tail -n +2 | awk '{print $1}' | sort -u; }

# rustruct-types の直接依存は serde と rust_decimal（JIS の丸め、docs/materials/adr/0002）だけ
got=$(direct_deps rustruct-types | tr '\n' ' ')
if [ "$got" != "rust_decimal rust_decimal_macros serde " ]; then
  echo "NG: rustruct-types の直接依存が docs/stack.md（serde、rust_decimal）と違う: $got"; fail=1
fi

# rustruct-calc の直接依存は rustruct-types だけ
got=$(direct_deps rustruct-calc)
if [ "$got" != "rustruct-types" ]; then
  echo "NG: rustruct-calc の直接依存が docs/stack.md（rustruct-types のみ）と違う:"; echo "$got"; fail=1
fi

# rustruct-calc は I/O も非同期ランタイムも依存木に持たない
if tree rustruct-calc | grep -Eq '^(tokio|async-std|smol|futures|reqwest|hyper|axum|mio|rustls|openssl) '; then
  echo "NG: rustruct-calc の依存木に I/O / 非同期ランタイムがある"; tree rustruct-calc; fail=1
fi

# charta は rustruct-types に依存しない（木全体）
if tree charta | grep -q '^rustruct-types '; then
  echo "NG: charta が rustruct-types に依存している"; tree charta; fail=1
fi

# numerus は依存を持たない
got=$(direct_deps numerus)
if [ -n "$got" ]; then
  echo "NG: numerus の直接依存が docs/stack.md（なし）と違う:"; echo "$got"; fail=1
fi

[ "$fail" -eq 0 ] && echo "OK: 依存の向きは docs/stack.md どおり"
exit "$fail"
