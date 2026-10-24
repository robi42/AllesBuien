# shellcheck shell=bash
#
# cargo-about lists the crates pkgbuild/about.toml lets it see, and the
# third-party notices gate holds the committed file to that list, so a crate
# about.toml leaves out, through a wrong target or an exclusion, is missing
# from both and passes there. This gate asks cargo tree what the binary links
# instead. Dropping ring's entry from the committed index stands in for that
# omission, since the index reads the same either way. ring is named rather
# than read out of the file because the index also lists procedural macros and
# their dependencies, which the binary does not link and this gate does not
# ask about; ring is linked for as long as rustls uses its provider.

gate="third-party notice coverage"
script="bin/test"
outcome="red"
expect=("pkgbuild/THIRD-PARTY-LICENSES lists no entry for ring, which allesbuien links")

mutate() {
  local entry
  entry=$(awk '
    listing && NF == 0 { exit }
    listing && $1 == "ring" { print FNR; exit }
    prev == "Crates" && /^-+$/ { listing = 1 }
    { prev = $0 }
  ' pkgbuild/THIRD-PARTY-LICENSES)
  : "${entry:?pkgbuild/THIRD-PARTY-LICENSES lists no ring under Crates}"
  sed -i "${entry}d" pkgbuild/THIRD-PARTY-LICENSES
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
