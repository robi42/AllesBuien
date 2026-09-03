# shellcheck shell=bash
# shellcheck disable=SC2034  # bin/check-gates sources this file and reads these
#
# A workflow with a misspelled key is accepted by YAML and ignored by GitHub, so
# the setting simply never applies and the run looks fine. actionlint is what
# turns that into an error, and a misspelled job key is the cheapest way to
# prove it is still looking.

gate="workflow syntax"
script="bin/test"
outcome="red"
expect=(".github/workflows/violation.yml" 'unexpected key "unexpected_key"')

mutate() {
  {
    printf 'name: violation\n'
    printf 'on: push\n'
    printf 'jobs:\n'
    printf '  violation:\n'
    printf '    runs-on: ubuntu-latest\n'
    printf '    unexpected_key: true\n'
    printf '    steps:\n'
    printf '      - run: echo hi\n'
  } >.github/workflows/violation.yml
}
