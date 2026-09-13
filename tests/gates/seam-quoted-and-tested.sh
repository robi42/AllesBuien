# shellcheck shell=bash
#
# Prose that names a constructor, and a test module that builds a failing HTTP
# client to manufacture an error - core/src/retry.rs and core/src/status.rs both
# do exactly that. A seam gate that refused either would be weakened by the
# first person it blocked.

gate="side-effect seams"
script="bin/check-fast"
outcome="green"
expect=()

mutate() {
  cat >core/src/violation_seam_prose.rs <<'RUST'
// Never call Command::new( or env::var here; both have one home each.
pub const A: u8 = 1;

#[cfg(test)]
mod tests {
    fn client() {
        let _ = reqwest::Client::new();
    }
}
RUST
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
