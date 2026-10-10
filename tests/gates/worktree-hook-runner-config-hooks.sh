# shellcheck shell=bash
#
# The runner's commit looks for its hooks in /dev/null and turns off none of
# the hooks the config defines by name, which git 2.55.0 runs whatever
# core.hooksPath says: a hook.<name>.command for each event its
# hook.<name>.event lists. The gate's global config defines a pre-commit that
# fails any commit, and the gate's report carries that hook's complaint.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "a commit ran a hook the global config defines: pre-commit")

mutate() {
  sed -i -E 's/-c hook\.[a-z-]+\.enabled=false ?//g' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
