# shellcheck shell=bash
#
# The runner clears GIT_DIR and the index but keeps the commit's -c options,
# which git hands the hook as GIT_CONFIG_PARAMETERS, so every git command in the
# scratch runs with them: given a hooks path, git init runs the hooks there, as
# add and status do, and a commit that names no hooks directory of its own runs
# the very hook that started the run.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "ran git with the -c options it was handed" "reference-transaction")

mutate() {
  sed -i 's/^unset -v .*/unset -v GIT_DIR GIT_INDEX_FILE GIT_PREFIX/' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
