# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The gates are shell, and this repository has already shipped a shell bug that
# disabled a gate while reporting success. shellcheck is the only reader that
# checks the checkers, so a shellcheck that had quietly stopped running would
# take the rest of them with it.

gate="shell scripts"
script="bin/test"
outcome="red"
expect=("bin/violation-unquoted" "SC2086")

mutate() {
  # shellcheck disable=SC2016  # the unexpanded $1 and $message are the defect
  printf '#!/usr/bin/env bash\nmessage=$1\necho $message\n' >bin/violation-unquoted
}
