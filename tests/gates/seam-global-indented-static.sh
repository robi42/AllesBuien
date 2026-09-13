# shellcheck shell=bash
#
# The same process-global counter, one rustfmt level inside an inline module. A
# rule anchored at column zero of the raw line reads it as indentation followed
# by nothing it knows, and the global gets through.

gate="side-effect seams"
script="bin/check-fast"
outcome="red"
expect=("core/src/violation_nested_global.rs" "COUNTER")

mutate() {
  cat >core/src/violation_nested_global.rs <<'RUST'
pub mod inner {
    pub static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
}
RUST
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
