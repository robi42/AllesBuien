# shellcheck shell=bash
#
# An entry point the manifest names but the tree does not hold installs,
# enables, and draws nothing: the shell skips the widget and says so only on its
# own console. Deleting the file leaves plugin/ with no QML at all, which also
# proves the colour gate reads an empty directory as nothing rather than as a
# literal glob grep cannot open - that gate staying green is part of this
# fixture passing, since the runner allows exactly one FAIL headline.

gate="omarchy plugin"
script="bin/check-fast"
outcome="red"
expect=("plugin/BarWidget.qml" "not a tracked file")

mutate() {
  rm plugin/BarWidget.qml
  [[ ! -e plugin/BarWidget.qml ]]
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
