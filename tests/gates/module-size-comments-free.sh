# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The budget counts behaviour, never explanation. This file is 751 physical
# lines and exactly 250 pure ones, so a gate built on wc -l would refuse it and
# this one must not. That property is what makes "split the module" the only way
# to satisfy the budget: joining statements, deleting blank lines and dropping
# doc comments - every compression trick a naive count rewards - move the number
# by zero.

gate="module size budget"
script="bin/check-fast"
outcome="green"
expect=()

mutate() {
  {
    printf '//! Exactly at the budget, buried in comments and blank lines.\n'
    seq 1 250 | while read -r n; do
      printf '// what line %s is for\n\npub const VIOLATION_%s: u32 = %s;\n' "$n" "$n" "$n"
    done
  } >core/src/violation_wordy.rs
}
