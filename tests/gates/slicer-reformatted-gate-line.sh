# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# Not a gate's fixture but the runner's. bin/check-gates checks that a gate
# still exists with `grep -qF`, a substring match, and then cuts its block out
# of bin/test with awk on `$0 == want`, a whole-line match. A header that has
# been reformatted - a trailing comment is enough - satisfies the first and
# matches nothing under the second, so the slice is the preamble alone: it runs
# no gate and exits 0.
#
# That is not a silent pass. The verdict comparison catches it and the run goes
# red. What it is is a red that blames the wrong thing: the message says the
# formatting gate stopped refusing unformatted code, and the formatting gate was
# never run. This fixture pins the message that says whose defect it is.
#
# The formatting gate is the target because it is the cheapest block to slice
# and its baseline is already paid for by formatting-unformatted. Nothing here
# reaches cargo - the guard fires before the slice is executed.

gate="formatting"
script="bin/test"
outcome="red"
expect=("slice_gate cut no block for gate 'formatting'")

mutate() {
  sed -i 's|^echo "==> gate: formatting"$|&  # reformatted|' bin/test
}
