# shellcheck shell=bash
#
# Public, not column-0 private: a visibility prefix is the spelling a rule that
# anchored on a bare `static` would miss, and a process-global counter is shared
# state every test in the binary races over.

gate="side-effect seams"
script="bin/check-fast"
outcome="red"
expect=("core/src/violation_global.rs" "COUNTER")

mutate() {
  printf 'pub static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);\n' >core/src/violation_global.rs
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
