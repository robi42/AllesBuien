# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# A second GUI test in one harness shares the process with the first, so it meets
# a QGuiApplication that already exists and the run dies on SIGSEGV - a crash
# that reads like a product bug rather than a filing mistake. Cheaper to refuse
# the second #[test] than to debug the segfault.

gate="one GUI test per harness"
script="bin/check-fast"
outcome="red"
expect=("has 2 #[test] functions, expected 1")

mutate() {
  local harness
  harness=$(git ls-files 'app/tests/*.rs' | grep -E '^app/tests/[^/]+\.rs$' | awk 'NR == 1')
  printf '\n#[test]\nfn violation_second_gui_test() {}\n' >>"$harness"
  expect+=("$harness")
}
