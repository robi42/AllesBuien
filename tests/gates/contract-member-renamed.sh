# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# Layer three, and the only check that would have gone red on either real
# regression: a member the UI reads that the registry no longer answers to. The
# rename here stays camelCase on purpose, so layer two cannot see it and the red
# has to come from the contract itself. Which member is renamed is read from the
# QML rather than pinned, so the fixture keeps working as the UI grows.

gate="qml member resolution"
script="bin/check-qml-contract"
outcome="red"
expect=("QML reads members the type registry does not define")
args=(violation.qmltypes)

mutate() {
  local member
  cp target/cxxqt/qml_modules/nl/allesbuien/plugin.qmltypes violation.qmltypes
  member=$(grep -ohE '\b(weather|radar|theme)(Controller|Source)\.[A-Za-z_][A-Za-z0-9_]*' app/qml/*.qml |
    sed 's/.*\.//' | sort -u | awk 'NR == 1')
  sed -i "s/name: \"$member\"/name: \"${member}Elsewhere\"/" violation.qmltypes
  expect+=("$member")
}
