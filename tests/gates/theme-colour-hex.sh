# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# A hex literal anywhere but Theme.qml is a value that stops following the
# Omarchy theme the moment the theme changes, and it never looks wrong in the
# palette it was written against. Nothing at runtime complains; the window just
# keeps one colour from the old theme.

gate="theme-owned colours"
script="bin/check-fast"
outcome="red"
expect=("app/qml/ViolationSwatch.qml" "#ff0000")

mutate() {
  printf 'import QtQuick\n\nRectangle {\n    color: "#ff0000"\n}\n' >app/qml/ViolationSwatch.qml
}
