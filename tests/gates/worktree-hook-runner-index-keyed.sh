# shellcheck shell=bash
#
# The runner clears git's variables only when GIT_INDEX_FILE is set, as if a
# commit's hook were the only thing that hands them over. git rebase -x
# bin/test in a linked worktree hands it GIT_DIR alone, so the scratch's git
# init marks the worktree's repository bare and its commit lands on the
# worktree's branch.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "as git rebase -x runs it in a linked worktree" "core.bare is now true" "refs/heads/task")

mutate() {
  # The runner's one column-one unset is the line that clears them.
  sed -i "s/^unset -v .*/[[ -z \${GIT_INDEX_FILE:-} ]] || &/" bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
