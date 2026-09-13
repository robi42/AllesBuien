# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# Coverage percentages are only comparable across runs if every run measures
# them with the same tool, so the pin is what makes the 90% floor mean anything.
# Writing this fixture is what showed the gate said nothing when it failed: it
# was a `| grep -q` whose whole failure output was an exit code, and the check
# now names the installed version and the pinned one.

gate="cargo-llvm-cov version pin"
script="bin/test"
outcome="red"
expect=(".llvm-cov-version pins 0.0.0-violation")

mutate() {
  printf '0.0.0-violation\n' >.llvm-cov-version
}
