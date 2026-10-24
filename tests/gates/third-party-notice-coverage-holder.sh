# shellcheck shell=bash
#
# cargo-about copies a crate's licence files and never reads the headers of
# its source files, so a copyright holder named only in an ISC header reaches
# the notices through pkgbuild/about.hbs's hand-kept section or not at all,
# and the third-party notices gate cannot tell which. Deleting the section's
# first holder line, read out of the committed file with its crate, stands in
# for a header whose holder nobody added: the gate has to name the crate and
# the holder its headers carry. The holder is the line with its years taken
# off, as the gate takes them off the headers.

gate="third-party notice coverage"
script="bin/test"
outcome="red"
expect=("leaves out notices that code allesbuien links asks for")

mutate() {
  local found line crate holder
  found=$(awk '
    section && /^[^ ]+  Copyright / { print FNR "\t" $0; exit }
    $0 == "Notices the licence texts below do not carry" { section = 1 }
  ' pkgbuild/THIRD-PARTY-LICENSES)
  : "${found:?pkgbuild/THIRD-PARTY-LICENSES names no copyright holder under its hand-kept heading}"
  line=${found#*$'\t'}
  crate=${line%%  *}
  holder=$(sed -E 's/^[^ ]+  Copyright[[:space:]]*(\([Cc]\)|©)?[[:space:]]*[0-9][-0-9, ]*//; s/[.,[:space:]]+$//' <<<"$line")
  sed -i "${found%%$'\t'*}d" pkgbuild/THIRD-PARTY-LICENSES
  expect+=("$crate: " "names $holder")
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
