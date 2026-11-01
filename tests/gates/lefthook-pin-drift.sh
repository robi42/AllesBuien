# shellcheck shell=bash
#
# mise.toml's lefthook moved without lefthook.yml's min_version, so the hook
# would accept a lefthook the pin no longer names.

gate="lefthook pin"
script="bin/check-fast"
outcome="red"
expect=("lefthook.yml: 2.2.2, mise.toml: 2.1.6")

mutate() {
  sed -i 's/^lefthook = .*/lefthook = "2.1.6"/' mise.toml
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
