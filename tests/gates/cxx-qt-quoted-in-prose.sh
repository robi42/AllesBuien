# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The false positive the same comment-awareness prevents. Two bridge module docs
# already quote both literals verbatim, so a gate that counted occurrences reads
# six blocks against five attributes on a clean tree and refuses it. A third
# quoting module doc, in a core file that has no bridge at all, must stay
# invisible - explaining the rule can never be what breaks it.

gate="cxx-qt name binding"
script="bin/check-fast"
outcome="green"
expect=()

mutate() {
  local module
  module=$(git ls-files 'core/src/*.rs' | awk 'NR == 1')
  {
    printf '//! Prose that quotes extern "RustQt" and #[auto_cxx_name] verbatim,\n'
    printf '//! the way two bridge docs in this tree already do.\n'
    cat "$module"
  } >"$module.violation"
  mv "$module.violation" "$module"
}
