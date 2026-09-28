# shellcheck shell=bash
#
# The bar widget is drawn inside omarchy-shell and has to follow its theme as
# the window follows Omarchy's, so the colour gate reads plugin/ as well as
# app/qml. A widened file list nobody has watched fail is a decoration: this is
# the hex fixture again, one directory over.

gate="theme-owned colours"
script="bin/check-fast"
outcome="red"
expect=("plugin/ViolationSwatch.qml" "#ff0000")

mutate() {
  printf 'import QtQuick\n\nRectangle {\n    color: "#ff0000"\n}\n' >plugin/ViolationSwatch.qml
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
