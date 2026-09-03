# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# rustfmt --check is the one gate here nobody doubts, which is precisely why it
# is worth a fixture: it is also the one whose failure output is a diff rather
# than a sentence, so "names the offender" means the path has to appear in that
# diff header.

gate="formatting"
script="bin/test"
outcome="red"
expect=("Diff in")

mutate() {
  local module
  module=$(git ls-files 'core/src/*.rs' | awk 'NR == 1')
  printf '\nfn violation_formatting( value:u32 )->u32{value+1}\n' >>"$module"
  expect+=("$module")
}
