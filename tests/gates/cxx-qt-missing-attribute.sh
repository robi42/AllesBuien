# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The regression this gate exists for, reproduced: an extern "RustQt" block that
# lost its #[auto_cxx_name]. It has happened twice, and both times nothing failed
# to compile, no test failed, and the window simply came up unthemed and unbound
# because every multi-word member kept its snake_case Rust name. The gate has to
# name the file and the line, because "some block is unbound" is not actionable
# in a tree with four bridges.

gate="cxx-qt name binding"
script="bin/check-fast"
outcome="red"
expect=('extern "RustQt"')

mutate() {
  local bridge
  bridge=$(git ls-files '*.rs' | xargs -r grep -lF '#[auto_cxx_name]' | awk 'NR == 1')
  sed -i '/^[[:space:]]*#\[auto_cxx_name\][[:space:]]*$/d' "$bridge"
  expect+=("$bridge")
}
