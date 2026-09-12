# shellcheck shell=bash
#
# reqwest::Client::new() rather than the builder, deliberately: the ordinary
# constructor is the spelling a rule that only named the builder would miss, and
# either one outside the three transports is a live HTTP client no FeedApi or
# RadarApi fake stands in for.

gate="side-effect seams"
script="bin/check-fast"
outcome="red"
expect=("core/src/violation_http.rs" "reqwest::Client::new")

mutate() {
  printf 'pub fn client() -> reqwest::Client {\n    reqwest::Client::new()\n}\n' >core/src/violation_http.rs
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
