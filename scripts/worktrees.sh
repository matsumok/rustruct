#!/usr/bin/env sh
# 作業用のフォルダ（git worktree）を大きさと状態つきで並べ、名前を挙げたものを消す。
#   sh scripts/worktrees.sh            並べる（主のフォルダは除く）
#   sh scripts/worktrees.sh rm <名前>…  消す。名前は .claude/worktrees/ の下のフォルダ名か、フォルダのパス
# 並べる列: 大きさ（target を含む）、ブランチ、状態
#   取り込み済み = HEAD が origin/main に含まれる。切ったばかりで何もコミットしていないフォルダもこれになるので、
#                  それだけで消してよいとは限らない（別のセッションが使っているかもしれない）
#   変更あり / ロック中 = 消さない。ロックは Claude Code が使っている印
# rm は「取り込み済み・変更なし・ロックなし」の 3 つが揃ったものだけ消し、揃わなければ理由を言って断る。
# ブランチは消さない（取り込み済みなら git branch -d で消せる）。origin/main は最後に fetch した時点のもの。
# rm はサンドボックスの外で動かす（.git/worktrees がサンドボックスの中では書けない）。
set -u
cd "$(dirname "$0")/.."
main=$(git rev-parse --path-format=absolute --git-common-dir | sed 's#/\.git$##')

state() {
  w=$1
  s=
  git -C "$w" merge-base --is-ancestor HEAD origin/main 2>/dev/null && s="取り込み済み"
  [ -n "$(git -C "$w" status --porcelain 2>/dev/null)" ] && s="${s:+$s・}変更あり"
  [ -f "$(git -C "$w" rev-parse --git-dir)/locked" ] && s="${s:+$s・}ロック中"
  printf '%s' "${s:-未取り込み}"
}

worktrees() {
  git worktree list --porcelain | awk '/^worktree /{print substr($0, 10)}' | grep -vxF "$main"
}

if [ "${1:-}" = rm ]; then
  shift
  [ "$#" -gt 0 ] || { echo "消す名前を挙げる（sh scripts/worktrees.sh で一覧）" >&2; exit 1; }
  fail=0
  for name in "$@"; do
    case "$name" in /*) w=$name ;; *) w=$main/.claude/worktrees/$name ;; esac
    if ! worktrees | grep -qxF "$w"; then echo "$name: 作業用のフォルダではない" >&2; fail=1; continue; fi
    s=$(state "$w")
    if [ "$s" != "取り込み済み" ]; then echo "$name: 消さない（$s）" >&2; fail=1; continue; fi
    git worktree remove "$w" && echo "$name: 消した"
  done
  exit "$fail"
fi

worktrees | while read -r w; do
  size=$(du -sh "$w" 2>/dev/null | cut -f1)
  branch=$(git -C "$w" branch --show-current)
  printf '%6s  %-28s  %-32s  %s\n' "$size" "${w#"$main"/.claude/worktrees/}" "${branch:-(detached)}" "$(state "$w")"
done
