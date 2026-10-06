# shellcheck shell=bash
#
# The runner clears git's variables only when GIT_DIR is set, as if a linked
# worktree were the one place they could name another repository. In the main
# checkout git sets no GIT_DIR, but a commit with -a, or of named paths, hands
# the hook the commit's own index by its absolute path, and the scratch's git
# add writes the scratch's files into it: the commit then fails on an object
# only the scratch holds. The gate names the decoy's copy of that index.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "in the main checkout" "next-index-1.lock")

mutate() {
  # The runner's one column-one unset is the line that clears them.
  sed -i "s/^unset -v .*/[[ -z \${GIT_DIR:-} ]] || &/" bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
