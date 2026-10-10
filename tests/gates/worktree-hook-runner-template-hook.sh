# shellcheck shell=bash
#
# The runner's commit reads no global config, in place of looking for its hooks
# in /dev/null: it meets no core.hooksPath then, but the hooks an
# init.templateDir put into the scratch at git init are still there, since that
# git init read the global config, and the commit runs them. The gate's global
# config has an init.templateDir whose hooks fail any commit, and the gate's
# report carries their complaint, which names the hook.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "a commit ran a hook git init copied from the global config: pre-commit")

mutate() {
  sed -i 's|^git -C .* \\$|GIT_CONFIG_GLOBAL=/dev/null &|; s|-c core.hooksPath=/dev/null ||' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
