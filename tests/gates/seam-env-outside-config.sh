# shellcheck shell=bash
#
# An environment read outside config.rs is a setting no EnvSource fake can
# supply, so a test either mutates the process environment or cannot reach the
# branch at all.

gate="side-effect seams"
script="bin/check-fast"
outcome="red"
expect=("core/src/violation_env.rs" "env::var")

mutate() {
  printf 'pub fn home() -> Option<String> {\n    std::env::var("HOME").ok()\n}\n' >core/src/violation_env.rs
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
