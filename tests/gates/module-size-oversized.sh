# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# One line over the budget has to be refused, and refused by name: a gate that
# says "some file is too big" leaves the reader to find it. 251 constants is the
# smallest violation there is, which also pins the boundary from above - the
# sibling fixture sits at exactly 250 and stays green.

gate="module size budget"
script="bin/check-fast"
outcome="red"
expect=("core/src/violation_bloat.rs" "251" "(limit 250)")

mutate() {
  {
    printf '//! One pure line over the budget, and nothing else.\n'
    seq 1 251 | while read -r n; do
      printf 'pub const VIOLATION_%s: u32 = %s;\n' "$n" "$n"
    done
  } >core/src/violation_bloat.rs
}
