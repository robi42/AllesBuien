# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# The false positive a looser scan would produce. Every one of these scripts is
# mostly prose, and prose about gates quotes gate headers - this file does it
# twice above. A gate that grepped for the string anywhere on the line would
# read a comment explaining a retired gate as a live declaration and demand a
# fixture for something that does not exist, and the first person it blocked
# would delete it. Column one is the definition, exactly as the slicer in
# bin/check-gates already has it, and a comment is invisible to both.

gate="gate coverage"
script="bin/check-fast"
outcome="green"
expect=()

mutate() {
  {
    printf '\n'
    printf '# A comment quoting echo "==> gate: retired" verbatim, the way a note\n'
    printf '# about a gate that used to be here would have to.\n'
  } >>bin/check-fast
}
