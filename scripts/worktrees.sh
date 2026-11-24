#!/usr/bin/env sh
# 作業用のフォルダ（git worktree）を大きさと状態つきで並べ、名前を挙げたものを消す。取り込み済みのブランチも消す。
#   sh scripts/worktrees.sh            並べる（主のフォルダは除く）
#   sh scripts/worktrees.sh rm <名前>…  消す。名前は .claude/worktrees/ の下のフォルダ名か、フォルダのパス。ブランチも消す
#   sh scripts/worktrees.sh prune      git fetch --prune のあと、origin/main に入っていて、どのフォルダでも使っていない
#                                      ローカルのブランチを消す（main は残す）。サブエージェントの worktree-agent-* もこれで消える
# 並べる列: 大きさ（target を含む）、ブランチ、状態
#   取り込み済み = HEAD が origin/main に含まれる。切ったばかりで何もコミットしていないフォルダもこれになるので、
#                  それだけで消してよいとは限らない（別のセッションが使っているかもしれない）
#   変更あり / ロック中 = 消さない。ロックは Claude Code が使っている印
# rm は「取り込み済み・変更なし・ロックなし」の 3 つが揃ったものだけ消し、揃わなければ理由を言って断る。
# rm の origin/main は最後に fetch した時点のもの。
# rm と prune はサンドボックスの外で動かす（.git/worktrees と refs がサンドボックスの中では書けない）。
# ブランチを消すと .git/config が書けないという警告が出るが、ブランチは消える（追跡の設定が残るだけ）。
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
    branch=$(git -C "$w" branch --show-current)
    git worktree remove "$w" || { fail=1; continue; }
    echo "$name: 消した"
    if [ -n "$branch" ] && [ "$branch" != main ]; then
      git branch -D "$branch" 2>&1 | grep -v -e config -e '^hint:'
    fi
  done
  exit "$fail"
fi

if [ "${1:-}" = prune ]; then
  git fetch -q --prune origin
  used=$(git worktree list --porcelain | awk '/^branch /{sub("refs/heads/", "", $2); print $2}')
  git for-each-ref refs/heads --format='%(refname:short)' | while read -r b; do
    [ "$b" = main ] && continue
    printf '%s\n' "$used" | grep -qxF "$b" && continue
    git merge-base --is-ancestor "$b" origin/main || continue
    git branch -D "$b" 2>&1 | grep -v -e config -e '^hint:'
  done
  exit 0
fi

worktrees | while read -r w; do
  size=$(du -sh "$w" 2>/dev/null | cut -f1)
  branch=$(git -C "$w" branch --show-current)
  printf '%6s  %-28s  %-32s  %s\n' "$size" "${w#"$main"/.claude/worktrees/}" "${branch:-(detached)}" "$(state "$w")"
done
