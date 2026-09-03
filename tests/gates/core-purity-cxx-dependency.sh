# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The gate that was provably broken. It read
#   cargo tree | grep -qi cxx && exit 1 || true
# and under pipefail that pipeline reports success precisely when it should
# fail: grep exits at the first match, the producer takes SIGPIPE, the status
# becomes 141, `&& exit 1` never fires and `|| true` swallows what is left. It
# only ever looked like it worked because cargo tree's output fits the pipe
# buffer. Nothing about a green run distinguished it from a gate that was
# checking - which is the argument for this whole directory.

gate="core purity"
script="bin/test"
outcome="red"
expect=("allesbuien-core gained a CXX/Qt dependency" "cxx v")

mutate() {
  awk '{ print } /^\[dependencies\]$/ { print "cxx = \"1\"" }' core/Cargo.toml \
    >core/Cargo.toml.violation
  mv core/Cargo.toml.violation core/Cargo.toml
}
