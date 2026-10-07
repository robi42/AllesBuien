# shellcheck shell=bash
#
# The runner's commit takes its hooks from wherever the global config says: a
# core.hooksPath there, or the hooks an init.templateDir put into the scratch
# at git init, neither of which clearing git's local environment touches. A
# global lefthook setup would run the scratch's own lefthook.yml there. The gate
# runs every script under a global config of its own, whose hook fails any
# commit that runs it, and the gate's report carries that hook's complaint.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "a commit ran the hook the global config names: pre-commit")

mutate() {
  sed -i 's|-c core.hooksPath=/dev/null ||' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
