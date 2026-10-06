# shellcheck shell=bash
#
# The runner clears git's variables before it reads this repository rather
# than after. Run from a hook of git commit -a, or of git commit with paths, it
# then reads the worktree's own index instead of the commit's, which only
# GIT_INDEX_FILE names, and judges a tree nobody is committing. The decoy's own
# index lists a file the working tree no longer has, so the copy cannot stat it.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "plugin/Ghost.qml")

mutate() {
  sed -i '0,/^git ls-files -z --cached/s//unset -v GIT_DIR GIT_INDEX_FILE\n&/' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
