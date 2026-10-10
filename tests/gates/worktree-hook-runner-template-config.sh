# shellcheck shell=bash
#
# The runner's commit reads no global config, in place of turning off the
# hooks a config defines by name: it meets none the global config defines
# then, but the scratch's git init read the global config and copied the
# config its init.templateDir holds into the scratch's own, and a hook that
# config defines runs whatever core.hooksPath says. The gate's template has a
# config that defines a pre-commit that fails any commit, and the gate's
# report carries that hook's complaint.

gate="worktree hook"
script="bin/check-worktree-hook"
outcome="red"
expect=("FAIL bin/check-gates" "a commit ran a hook git init copied into the repository config: pre-commit")

mutate() {
  sed -i 's|^git -C .* \\$|GIT_CONFIG_GLOBAL=/dev/null &|' bin/check-gates
  sed -i -E 's/-c hook\.[a-z-]+\.enabled=false ?//g' bin/check-gates
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
