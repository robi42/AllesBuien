# shellcheck shell=bash
#
# The plugin marketplace takes a root preview.avif as readily as a preview.png,
# and a format missing from the gate's extension list is an image the allowlist
# never sees: an AVIF would land unasked, whatever it depicted.

gate="image allowlist"
script="bin/check-fast"
outcome="red"
expect=("tracked but not on the allowlist: preview.avif")

mutate() {
  printf 'stand-in bytes for an AVIF preview\n' >preview.avif
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
