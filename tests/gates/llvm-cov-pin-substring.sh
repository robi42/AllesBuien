# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The gate compares cargo-llvm-cov's whole report with the version
# .llvm-cov-version pins. It used to ask only whether the report contained the
# pin, which let a pin of 1.2.3 pass a 1.2.30 found on PATH. The report names
# the tool whatever its version, so the tool's own name, planted as the pin, is
# in every report and is no release's version: a substring match takes it, and
# the fixture does not have to move when the pin does.

gate="cargo-llvm-cov version pin"
script="bin/test"
outcome="red"
expect=(".llvm-cov-version pins cargo-llvm-cov")

mutate() {
  printf 'cargo-llvm-cov\n' >.llvm-cov-version
}
