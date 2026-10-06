# shellcheck shell=bash
#
# The runner clears git's variables only when GIT_DIR or GIT_INDEX_FILE is set,
# as if those two were the only ones that reach another repository. git -c ...
# rebase -x in the main checkout hands what it runs neither, only the -c
# options, so the scratch's commit runs with them, a hooks path among them.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "rebase -x runs it in the main checkout" "made a commit with the -c options it was handed")

mutate() {
  # The runner's one column-one unset is the line that clears them.
  sed -i "s/^unset -v .*/[[ -z \${GIT_DIR:-}\${GIT_INDEX_FILE:-} ]] || &/" bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
