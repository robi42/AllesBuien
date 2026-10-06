# shellcheck shell=bash
#
# The hook's own script, given the first line a gate needs before it can run
# git in a repository of its own: a scratch directory, and git init in it.
# bin/check-fast reads tracked text and nothing else today, so this is the
# defect it does not have. It pins that the gate would see the day it did,
# since lefthook runs bin/check-fast on every commit, in every worktree, and
# there git init in a scratch marks the worktree's repository bare.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-fast" "core.bare is now true")

mutate() {
  local line="probe=\$(mktemp -d) && git -C \"\$probe\" init -q && rm -rf \"\$probe\""
  sed -i "2a $line" bin/check-fast
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
