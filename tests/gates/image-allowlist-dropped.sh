# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The allowlist is a two-way contract, and this is the direction that rots
# quietly: an entry naming a file nobody ships any more. Left unchecked the list
# fills with names that assert nothing, and the next reader cannot tell which
# entries are still load-bearing.

gate="image allowlist"
script="bin/check-fast"
outcome="red"
expect=("on the allowlist but no longer tracked: preview.png")

mutate() {
  rm preview.png
}
