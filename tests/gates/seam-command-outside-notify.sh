# shellcheck shell=bash
#
# A notify-send spawned anywhere but core/src/notify.rs is a desktop side effect
# no Notifier fake can intercept, so the test that injects one stays green while
# the real process fires on every run.

gate="side-effect seams"
script="bin/check-fast"
outcome="red"
expect=("core/src/violation_seam.rs" "Command::new")

mutate() {
  printf 'pub fn ping() {\n    let _ = std::process::Command::new("notify-send");\n}\n' >core/src/violation_seam.rs
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
