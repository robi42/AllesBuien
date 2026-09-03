# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The other half of reading the file as code rather than counting strings. The
# gate skips comment lines, which is what lets two module docs quote both
# literals verbatim without tripping it; the risk that buys is a commented-out
# attribute silently counting as a present one. Here the attribute is still
# spelled correctly and still sits directly above the block - it is just prose,
# and the binding it used to buy is gone.

gate="cxx-qt name binding"
script="bin/check-fast"
outcome="red"
expect=('extern "RustQt"')

mutate() {
  local bridge
  bridge=$(git ls-files '*.rs' | xargs -r grep -lF '#[auto_cxx_name]' | awk 'NR == 1')
  sed -i 's|^\([[:space:]]*\)#\[auto_cxx_name\]$|\1// #[auto_cxx_name]|' "$bridge"
  expect+=("$bridge")
}
