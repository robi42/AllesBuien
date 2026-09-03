# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The same two obligations are carried in the README, and a Markdown link is
# unusually easy to flatten by accident - a reflow, a table conversion, a
# copy-paste that keeps the words and drops the target. The words surviving is
# not enough: the terms ask for both.

gate="attribution"
script="bin/check-fast"
outcome="red"
expect=("README.md no longer carries the linked attribution")

mutate() {
  sed -i 's|\[Buienradar\.nl\](https://www\.buienradar\.nl)|Buienradar.nl|g' README.md
}
