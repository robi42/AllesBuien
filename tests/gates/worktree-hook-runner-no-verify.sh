# shellcheck shell=bash
#
# The runner's commit skips the hooks with --no-verify rather than looking for
# them in /dev/null. --no-verify skips pre-commit and commit-msg alone: the
# commit still runs prepare-commit-msg, post-commit, reference-transaction and
# post-index-change from the hooks directory the global config names. The
# gate's global config has a prepare-commit-msg that fails any commit too, and
# the gate's report carries that hook's complaint, which names it.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "a commit ran the hook the global config names: prepare-commit-msg")

mutate() {
  sed -i 's|-c core.hooksPath=/dev/null commit|commit --no-verify|' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
