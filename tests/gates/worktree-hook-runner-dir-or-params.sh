# shellcheck shell=bash
#
# The runner clears git's variables only when GIT_DIR or GIT_CONFIG_PARAMETERS
# is set, as if a linked worktree or a commit's -c options were the only way
# they could reach another repository. A commit with -a, or of named paths,
# made in the main checkout without -c options hands its hook GIT_INDEX_FILE
# alone, and the scratch's git add writes the scratch's files into the commit's
# index. Only that run hands GIT_INDEX_FILE without the other two, so this
# goes red only while the gate runs it without -c options. The gate names the
# decoy's copy of that index.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "as git runs the hook in the main checkout" "next-index-1.lock")

mutate() {
  # The runner's one column-one unset is the line that clears them.
  sed -i "s/^unset -v .*/[[ -z \${GIT_DIR:-}\${GIT_CONFIG_PARAMETERS:-} ]] || &/" bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
