# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# Buienradar's free-data terms name two visible obligations, and this is the
# first of them: the words Buienradar.nl are shown. It is a licence condition,
# so the gate that guards it is the one gate here whose failure to fail would be
# a legal problem rather than an engineering one.

gate="attribution"
script="bin/check-fast"
outcome="red"
expect=("app/qml/Main.qml no longer shows the visible text Buienradar.nl")

mutate() {
  sed -i '/text: "Buienradar.nl"/d' app/qml/Main.qml
}
