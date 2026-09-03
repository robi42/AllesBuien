# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# Buienradar's map artwork is licensed to Buienradar, and the cheapest way to
# vendor it is to drop a PNG in and forget. A blanket ban would be wrong - three
# images are legitimately ours to ship - so the gate carries an allowlist, and
# an allowlist is only worth having if an unlisted file is actually refused.

gate="image allowlist"
script="bin/check-fast"
outcome="red"
expect=("tracked but not on the allowlist: app/assets/buienradar-radar.png")

mutate() {
  printf 'stand-in bytes for vendored vendor artwork\n' >app/assets/buienradar-radar.png
}
