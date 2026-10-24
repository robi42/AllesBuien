# shellcheck shell=bash
#
# pkgbuild/about.toml's accepted list is the licence policy: a crate under a
# licence nobody accepted has to stop its notices being written, by name, rather
# than ship under terms nobody read. Taking a licence back out of the list
# stands in for a lock change that brings one in. The crate is the first in the
# committed file's index under a single licence other than Apache-2.0 or MIT,
# read out of the file, so it has nothing else to fall back on. The phrase
# expected is cargo-about's own, which the gate's message does not repeat.

gate="third-party notices"
script="bin/test"
outcome="red"
expect=("failed to satisfy license requirements")

mutate() {
  local pick crate licence
  pick=$(awk '
    listing && NF == 0 { exit }
    listing && NF == 2 && $2 != "Apache-2.0" && $2 != "MIT" { print; exit }
    prev == "Crates" && /^-+$/ { listing = 1 }
    { prev = $0 }
  ' pkgbuild/THIRD-PARTY-LICENSES)
  read -r crate licence <<<"$pick"
  : "${licence:?no crate under Crates in pkgbuild/THIRD-PARTY-LICENSES has a single licence to take away}"
  grep -qxF "  \"$licence\"," pkgbuild/about.toml
  awk -v gone="  \"$licence\"," '$0 != gone' pkgbuild/about.toml >pkgbuild/about.toml.violation
  mv pkgbuild/about.toml.violation pkgbuild/about.toml
  expect+=("$crate")
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
