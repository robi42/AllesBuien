# shellcheck shell=bash
#
# A second Tokio runtime outside app/src/services.rs owns threads nothing shuts
# down and futures the injected runtime never sees.

gate="side-effect seams"
script="bin/check-fast"
outcome="red"
expect=("app/src/violation_runtime.rs" "new_current_thread")

mutate() {
  printf 'pub fn runtime() -> std::io::Result<tokio::runtime::Runtime> {\n    tokio::runtime::Builder::new_current_thread().build()\n}\n' >app/src/violation_runtime.rs
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
