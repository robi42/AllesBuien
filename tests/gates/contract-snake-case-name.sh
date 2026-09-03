# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# Layer two of the name-binding contract, checked against what the build
# registered rather than what the source looks like. A block that lost
# #[auto_cxx_name] fills the registry with the Rust spelling, so any snake_case
# name in there is the symptom. station_name is the literal that was injected by
# hand the one time this gate was watched to fail; the artefact path is an
# argument precisely so it can be pointed at a deliberately broken copy.

gate="generated type names"
script="bin/check-qml-contract"
outcome="red"
expect=("station_name" "snake_case names reached the QML type registry")
args=(violation.qmltypes)

mutate() {
  cp target/cxxqt/qml_modules/nl/allesbuien/plugin.qmltypes violation.qmltypes
  printf '        name: "station_name"\n' >>violation.qmltypes
}
