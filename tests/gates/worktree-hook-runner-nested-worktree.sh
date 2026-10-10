# shellcheck shell=bash
#
# The runner copies a linked worktree checked out inside this one, which git
# lists as one untracked directory, .git file and all. That file names this
# repository, so the scratch's git status enters it and rewrites the nested
# worktree's index.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "worktrees/nested/index")

mutate() {
  sed -i "s#--exclude-standard | grep -zv '/\$' |#--exclude-standard |#" bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
