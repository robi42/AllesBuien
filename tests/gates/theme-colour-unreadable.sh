# shellcheck shell=bash
#
# The colour gate takes its files from the index, and the index can name a file
# the working tree no longer has: one staged and then deleted, or renamed, before
# the deletion is staged. grep cannot open it, and a scan that treated grep's
# read error like its "no match" would call a tree clean that it never read -
# the colour gate did exactly that in the first draft of its plugin/ widening.
#
# The runner stages the whole tree after mutate, which would stage the deletion
# too, so the fixture marks its own file assume-unchanged, and git add leaves
# the entry alone. The file is new to this fixture, so the runner's reset drops
# the entry, and the flag with it, before the next fixture runs.

gate="theme-owned colours"
script="bin/check-fast"
outcome="red"
expect=("plugin/Unstaged.qml" "went unscanned")

mutate() {
  printf 'import QtQuick\n\nItem {\n}\n' >plugin/Unstaged.qml
  git add plugin/Unstaged.qml
  git update-index --assume-unchanged plugin/Unstaged.qml
  rm plugin/Unstaged.qml
  [[ ! -e plugin/Unstaged.qml && -n $(git ls-files plugin/Unstaged.qml) ]]
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
