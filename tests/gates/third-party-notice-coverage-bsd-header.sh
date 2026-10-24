# shellcheck shell=bash
#
# futures-channel's src/mpsc/queue.rs is under Dmitry Vyukov's two-clause BSD
# licence, whose second clause asks a binary's documentation for the copyright
# notice, both conditions and the disclaimer, and cargo-about lists the crate
# under the Apache-2.0 it elects for the rest. A copyright line alone would
# not do, so the gate holds the notices to the header's whole text as well as
# to its holder. Deleting the reproduced header from the committed file has to
# trip both: the holder check and the text check each name the file.

gate="third-party notice coverage"
script="bin/test"
outcome="red"
expect=(
  "futures-channel: "
  "names Dmitry Vyukov"
  "has a BSD-style header whose text the notices do not reproduce"
)

mutate() {
  local block
  block=$(awk '
    !start && index($0, "Copyright") && index($0, "Dmitry Vyukov") { start = FNR }
    start && /of Dmitry Vyukov\.$/ { print start "," FNR; exit }
  ' pkgbuild/THIRD-PARTY-LICENSES)
  : "${block:?pkgbuild/THIRD-PARTY-LICENSES reproduces no licence header of Dmitry Vyukov}"
  sed -i "${block}d" pkgbuild/THIRD-PARTY-LICENSES
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
