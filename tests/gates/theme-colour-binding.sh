# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The sanctioned form, which the gate has to leave alone. A colour gate wide
# enough to refuse a controller binding would be unusable - every QML file in
# the tree writes one - and the first thing anyone would do is weaken it. Worth
# pinning that it does not.

gate="theme-owned colours"
script="bin/check-fast"
outcome="green"
expect=()

mutate() {
  printf 'import QtQuick\n\nRectangle {\n    color: appTheme.background\n}\n' >app/qml/ViolationBinding.qml
}
