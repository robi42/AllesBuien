# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The second obligation: those words link to the root domain. Losing the link
# while keeping the text is the plausible half-regression - the label still
# reads correctly on screen and nothing looks wrong - which is exactly why it
# needs its own fixture rather than sharing one with the text.

gate="attribution"
script="bin/check-fast"
outcome="red"
expect=("app/qml/Main.qml no longer links to https://www.buienradar.nl")

mutate() {
  sed -i 's|"https://www\.buienradar\.nl"|""|g' app/qml/Main.qml
}
