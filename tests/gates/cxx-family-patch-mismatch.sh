# shellcheck shell=bash
#
# cxx names every bridge symbol after its patch number and cxx-gen writes the
# C++ side of them, so a cxx-gen one patch away compiles cleanly and then fails
# to link eighteen minutes into bin/test. The versions are read out of the lock
# rather than pinned here, because Dependabot moves them every few weeks.

gate="cxx family lockstep"
script="bin/check-fast"
outcome="red"
expect=("Cargo.lock")

mutate() {
  local cxx_version next_patch
  cxx_version=$(awk 'hit { gsub(/"/, "", $3); print $3; exit } $0 == "name = \"cxx\"" { hit = 1 }' Cargo.lock)
  : "${cxx_version:?cxx is not in Cargo.lock}"
  next_patch=$((${cxx_version##*.} + 1))
  awk -v wanted="0.7.$next_patch" '
    hit && /^version = / { $0 = "version = \"" wanted "\"" }
    { hit = ($0 == "name = \"cxx-gen\""); print }
  ' Cargo.lock >Cargo.lock.violation
  mv Cargo.lock.violation Cargo.lock
  grep -qx "version = \"0.7.$next_patch\"" Cargo.lock
  expect+=("cxx $cxx_version" "cxx-gen 0.7.$next_patch")
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
