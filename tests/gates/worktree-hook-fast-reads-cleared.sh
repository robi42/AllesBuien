# shellcheck shell=bash
#
# The hook's own script clears git's variables before it reads this
# repository, as a script that runs git elsewhere has to. bin/check-fast runs
# git nowhere else, and with them cleared it reads the worktree's own index
# instead of the one git commit -a, or git commit with paths, hands the hook,
# and judges a tree nobody is committing. The decoy's own index lists a file
# the working tree no longer has, and the colour gate calls it unscanned.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-fast" "plugin/Ghost.qml" "went unscanned")

mutate() {
  sed -i '2a unset -v GIT_DIR GIT_INDEX_FILE' bin/check-fast
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
