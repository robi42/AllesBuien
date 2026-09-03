# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# A tag is a mutable reference, so whoever owns the action can move it under a
# workflow that holds this repository's contents. actionlint has no opinion on
# the subject, which is the whole reason this gate is written by hand - and a
# hand-written grep is exactly the kind that can stop matching without anyone
# noticing.

gate="pinned action SHAs"
script="bin/check-fast"
outcome="red"
expect=("@v5")

mutate() {
  local workflow
  workflow=$(git ls-files '.github/workflows/*.yml' | awk 'NR == 1')
  awk '!loosened && /uses: .*@[0-9a-f]+ #/ { sub(/@[0-9a-f]+ #.*/, "@v5"); loosened = 1 } { print }' \
    "$workflow" >"$workflow.violation"
  mv "$workflow.violation" "$workflow"
  expect+=("$workflow")
}
