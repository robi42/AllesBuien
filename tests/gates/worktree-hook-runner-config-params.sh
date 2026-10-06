# shellcheck shell=bash
#
# The runner clears GIT_DIR and the index but keeps the commit's -c options,
# which git hands the hook as GIT_CONFIG_PARAMETERS, so the scratch's commit
# runs with them: given a hooks path, it runs the hooks there, which can be the
# very hook that started the run.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "made a commit with the hook's -c options")

mutate() {
  sed -i 's/^unset -v .*/unset -v GIT_DIR GIT_INDEX_FILE GIT_PREFIX/' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
