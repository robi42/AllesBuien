# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The hole this gate was written to close. bin/check-gates counts the gates its
# fixtures named, not the gates that exist, so a gate shipped without a fixture
# never appears in its arithmetic: the summary keeps printing the same totals,
# every fixture still passes, and the new gate is a decoration nobody has ever
# watched fail. This repository has already shipped one of those.
#
# Appending the header after check-fast's own `exit` is deliberate: the gate has
# to be found by reading the file's shape, not by the appended line happening to
# run.

gate="gate coverage"
script="bin/check-fast"
outcome="red"
expect=("unproven decoration" "no fixture, and not named in tests/gates/uncovered")

mutate() {
  printf '\necho "==> gate: unproven decoration"\n' >>bin/check-fast
}
