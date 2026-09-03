# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The allowlist is a two-way contract, and this is the direction that rots
# quietly. An entry naming a gate nobody declares any more excuses nothing, and
# a list full of those is one nobody can read: the next person cannot tell which
# names are still load-bearing from which are debris, so the whole file stops
# being evidence. The same argument the image allowlist makes, about gates.

gate="gate coverage"
script="bin/check-fast"
outcome="red"
expect=("violation retired gate" "named in tests/gates/uncovered, but no script declares it")

mutate() {
  printf 'violation retired gate\n' >>tests/gates/uncovered
}
