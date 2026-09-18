# shellcheck shell=bash
#
# omarchy-plugin-validate refuses a symlink anywhere in the plugin folder, and
# `omarchy plugin add` clones this whole repository as that folder, so one
# convenience link at the root makes the release uninstallable; CLAUDE.md is a
# regular file importing AGENTS.md for exactly this reason. The runner's
# `git add --all` gives the link index mode 120000, which is what the gate
# reads.

gate="omarchy plugin"
script="bin/check-fast"
outcome="red"
expect=("CONTRIBUTING.md" "symlink")

mutate() {
  ln -s AGENTS.md CONTRIBUTING.md
  [[ -L CONTRIBUTING.md ]]
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
