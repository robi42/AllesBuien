# shellcheck shell=bash
#
# mise.toml's cargo-llvm-cov moved without .llvm-cov-version, so a machine mise
# set up would measure coverage with a different cargo-llvm-cov than CI does.
# The drifted version is one no release will ever carry, so the fixture does not
# have to move when the pin does, and the gate never runs cargo-llvm-cov, so no
# mise shim tries to resolve it.

gate="cargo-llvm-cov mise pin"
script="bin/test"
outcome="red"
expect=("mise.toml pins cargo-llvm-cov 0.0.0-drift but .llvm-cov-version pins")

mutate() {
  sed -i 's/^\("aqua:taiki-e\/cargo-llvm-cov" = \)".*"$/\1"0.0.0-drift"/' mise.toml
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
