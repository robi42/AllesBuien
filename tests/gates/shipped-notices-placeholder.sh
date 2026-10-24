# shellcheck shell=bash
#
# For a crate that ships no licence file, cargo-about writes the standard MIT
# text with "<year> <copyright holders>" for its copyright line and says
# nothing, and so does the third-party notices gate, since the committed file
# then matches what cargo-about writes. The line goes into the first MIT block
# of the committed file, where cargo-about would put it, and the crates that
# block covers are read out of the file, so the failure has to name them.

gate="shipped notices"
script="bin/check-fast"
outcome="red"
expect=("a copyright line that names nobody")

mutate() {
  local crates
  crates=$(awk '
    prev ~ /^=+$/ && $0 == "MIT License (MIT)" { block = 1 }
    block && $0 == "Used by:" { listing = 1; next }
    listing && /^  / { users = users (users == "" ? "" : ", ") substr($0, 3); next }
    listing { print users; exit }
    { prev = $0 }
  ' pkgbuild/THIRD-PARTY-LICENSES)
  : "${crates:?pkgbuild/THIRD-PARTY-LICENSES has no MIT block}"
  awk '
    prev ~ /^=+$/ && $0 == "MIT License (MIT)" { block = 1 }
    { print }
    block && !done && /^-+$/ { print "Copyright (c) <year> <copyright holders>"; done = 1 }
    { prev = $0 }
  ' pkgbuild/THIRD-PARTY-LICENSES >pkgbuild/THIRD-PARTY-LICENSES.violation
  mv pkgbuild/THIRD-PARTY-LICENSES.violation pkgbuild/THIRD-PARTY-LICENSES
  expect+=("for $crates: Copyright (c) <year> <copyright holders>")
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
