# shellcheck shell=bash
#
# Layer two looks for the Rust spelling a block hands C++ once it loses
# #[auto_cxx_name]. A cxx-qt-lib container alias is not that: from Qt 6.12,
# qmltyperegistrar registers a property's type by the alias it was declared
# with, so QList_f64 from weather_controller.rs reaches the registry. This pins
# the gate to letting such a name through.

gate="generated type names"
script="bin/check-qml-contract"
outcome="green"
args=(violation.qmltypes)

mutate() {
  cp target/cxxqt/qml_modules/nl/allesbuien/plugin.qmltypes violation.qmltypes
  printf '        name: "QList_f64"\n' >>violation.qmltypes
}

# bin/check-gates reads these after sourcing the file, which shellcheck cannot see.
: "$gate" "$script" "$outcome" "${args[@]}"
