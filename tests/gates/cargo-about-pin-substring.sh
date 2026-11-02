# shellcheck shell=bash
#
# The gate compares cargo-about's whole report with the version
# .cargo-about-version pins, where a substring match would let a pin of 1.2.3
# pass a 1.2.30. The report names the tool whatever its version, so the tool's
# own name, planted as the pin, is in every report and is no release's version:
# a substring match would take it, and the fixture does not have to move when
# the pin does.

gate="cargo-about version pin"
script="bin/test"
outcome="red"
expect=(".cargo-about-version pins cargo-about")

mutate() {
  printf 'cargo-about\n' >.cargo-about-version
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
