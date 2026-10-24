# shellcheck shell=bash
#
# pkgbuild/THIRD-PARTY-LICENSES is compared byte for byte with what cargo-about
# writes, and another release can word the same lock differently, so the
# comparison means something only against the cargo-about CI installs. The
# planted version is one no release will ever carry, so the fixture does not
# have to move when the pin does.

gate="cargo-about version pin"
script="bin/test"
outcome="red"
expect=(".cargo-about-version pins 0.0.0-violation")

mutate() {
  printf '0.0.0-violation\n' >.cargo-about-version
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
