# shellcheck shell=bash
#
# The gate compares cargo-llvm-cov's whole report with the version
# .llvm-cov-version pins. A looser comparison lets a version that
# extends the pin through, as 1.2.30 extends 1.2.3: a substring, a prefix, or a
# match after the report's space all do. Planting the pin with its last
# character taken off puts the installed version in that relation to it
# whatever the pin is, so every one of those forms takes it and the fixture
# moves with the pin.

gate="cargo-llvm-cov version pin"
script="bin/test"
outcome="red"
expect=(".llvm-cov-version pins")

mutate() {
  local pin
  pin=$(cat .llvm-cov-version)
  printf '%s\n' "${pin%?}" >.llvm-cov-version
  expect+=(".llvm-cov-version pins ${pin%?}")
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
