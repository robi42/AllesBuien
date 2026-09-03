# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# An unqualified identifier is the QML defect that costs a running window: it
# resolves to undefined, undefined is falsy, and every ternary downstream takes
# the wrong branch in silence. The strictness that catches it lives in the
# committed .qmllint.ini, and --max-warnings 0 is what makes a warning fatal, so
# this proves the pair still bites.

gate="qml lint"
script="bin/test"
outcome="red"
expect=("app/qml/ViolationLint.qml" "Unqualified access")

mutate() {
  {
    printf 'import QtQuick\n\n'
    printf 'Item {\n'
    printf '    property int violationWidth: undeclaredIdentifier.width\n'
    printf '}\n'
  } >app/qml/ViolationLint.qml
}
