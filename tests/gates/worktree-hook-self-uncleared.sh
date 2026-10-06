# shellcheck shell=bash
#
# The gate no longer clears git's local environment before it builds its
# decoy. bin/test runs it, and git rebase -x bin/test in a linked worktree
# hands it GIT_DIR, so its git init marks that worktree's repository bare. The
# copy of itself that the gate runs in the decoy's worktree does that to the
# decoy, and its commit lands on the worktree's branch.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-worktree-hook" "core.bare is now true" "refs/heads/task")

mutate() {
  # The gate's one column-one unset is the line that clears that environment.
  sed -i '/^unset -v /d' bin/check-worktree-hook
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
