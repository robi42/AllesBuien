# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The literal-shaped defect wearing a function call. Qt.rgba and the .lighter/
# .darker family produce a colour the theme never supplied, so banning only hex
# would leave the whole workaround open - and it is the more tempting one,
# because it reads like it is deriving a value rather than inventing one.

gate="theme-owned colours"
script="bin/check-fast"
outcome="red"
expect=("app/qml/ViolationTint.qml" "Qt.rgba")

mutate() {
  printf 'import QtQuick\n\nRectangle {\n    color: Qt.rgba(1, 0, 0, 1)\n}\n' >app/qml/ViolationTint.qml
}
