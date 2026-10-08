# shellcheck shell=bash
#
# The runner's commit turns off the hooks the config defines by name for
# pre-commit alone, as hook.pre-commit.enabled=false. A commit runs more events
# than that, and a hook defined for prepare-commit-msg still runs and can fail
# it. The gate's global config defines a prepare-commit-msg that fails any
# commit, and the gate's report carries that hook's complaint.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "a commit ran a hook the global config defines: prepare-commit-msg")

mutate() {
  sed -i -E 's/-c hook\.(prepare-commit-msg|commit-msg|post-commit|reference-transaction|post-index-change)\.enabled=false ?//g' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
