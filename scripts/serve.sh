#!/usr/bin/env sh
# dx serve を、渡したフォルダで立てる。作業用のフォルダ（git worktree）で見た目を確かめるための道具。
#   sh scripts/serve.sh [フォルダ] [ポート]
#   既定: このスクリプトのあるリポジトリ  8080
# サブエージェントのコマンドは毎回主のフォルダから始まり、cd は次のコマンドに残らない。cd と dx serve を && で
# つなぐと excludedCommands（前方一致）から外れてサンドボックスに戻る。このスクリプトは 1 つのコマンドで
# フォルダを決めて dx serve を立てるので、どこから呼んでもサンドボックスの外で走る。
# 撮るときは sh scripts/screenshot.sh http://localhost:<ポート>。
set -eu
dir=${1:-$(dirname "$0")/..}
port=${2:-8080}
[ -f "$dir/Cargo.toml" ] || { echo "$dir はリポジトリのフォルダではない" >&2; exit 1; }
# 前の dx serve が同じポートに残っていると、新しい方が別のフォルダを配っていると気づけない
if curl -s -m 2 -o /dev/null "http://localhost:$port"; then
  echo "localhost:$port に何かが動いている。止めるか、別のポートを渡す" >&2
  exit 1
fi
cd "$dir"
echo "dx serve: $(pwd) を localhost:$port で"
exec dx serve --port "$port"
