# shellcheck shell=bash
#
# A lock change that adds, drops or relicenses a crate changes what the package
# owes, and the notices it installs have to change with it. Dropping the first
# crate from the committed file's index stands in for one: what cargo-about
# writes from the untouched Cargo.lock still lists it, so the gate refuses the
# committed copy, and its diff names the crate. The entry is read out of the
# file rather than named here, so no dependency leaving the graph can make this
# fixture fail for the wrong reason.

gate="third-party notices"
script="bin/test"
outcome="red"
expect=("pkgbuild/THIRD-PARTY-LICENSES does not match Cargo.lock")

mutate() {
  local entry
  entry=$(awk 'listing { print; exit } prev == "Crates" && /^-+$/ { listing = 1 } { prev = $0 }' pkgbuild/THIRD-PARTY-LICENSES)
  : "${entry:?no crate is listed under Crates in pkgbuild/THIRD-PARTY-LICENSES}"
  awk -v drop="$entry" '$0 == drop && !done { done = 1; next } { print }' \
    pkgbuild/THIRD-PARTY-LICENSES >pkgbuild/THIRD-PARTY-LICENSES.violation
  mv pkgbuild/THIRD-PARTY-LICENSES.violation pkgbuild/THIRD-PARTY-LICENSES
  expect+=("+$entry")
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
