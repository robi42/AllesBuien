# shellcheck shell=bash
#
# The Omarchy manifest's version is what the marketplace lists and what the
# shell reports, and no Rust tool reads it, so a release that bumps the Cargo
# manifests and forgets this one ships a plugin announcing the previous version.
# Only the top-level key is rewritten: "schemaVersion" has to stay 1, or the
# omarchy plugin gate would go red beside this one.

gate="release version"
script="bin/check-fast"
outcome="red"
expect=("manifest.json" "9.9.9")

mutate() {
  awk '/^  "version":/ { $0 = "  \"version\": \"9.9.9\"," } { print }' manifest.json >manifest.json.violation
  mv manifest.json.violation manifest.json
  grep -qx '  "version": "9.9.9",' manifest.json
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
