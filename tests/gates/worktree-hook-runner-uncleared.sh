# shellcheck shell=bash
#
# The defect the gate was written for, put back. bin/check-gates no longer
# clears git's local environment before it fills its scratch repository, so,
# run as git runs a command in a linked worktree, its git init marks the
# worktree's repository bare and its commit lands on the worktree's branch -
# while the runner itself still reports every fixture green. The gate names the
# runner and the core.bare it left behind.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "core.bare is now true" "refs/heads/task")

mutate() {
  # The runner's one column-one unset is the line that clears that environment.
  sed -i '/^unset -v /d' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
