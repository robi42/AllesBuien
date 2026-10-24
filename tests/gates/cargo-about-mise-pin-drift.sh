# shellcheck shell=bash
#
# mise.toml's cargo-about moved without .cargo-about-version, so a machine mise
# set up would regenerate the notices with a different cargo-about than CI
# compares them with. The drifted version is one no release will ever carry,
# so the fixture does not have to move when the pin does, and the gate never
# runs cargo-about, so no mise shim tries to resolve it.

gate="cargo-about mise pin"
script="bin/test"
outcome="red"
expect=("mise.toml pins cargo-about 0.0.0-drift but .cargo-about-version pins")

mutate() {
  sed -i 's/^\("github:EmbarkStudios\/cargo-about" = \)".*"$/\1"0.0.0-drift"/' mise.toml
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
