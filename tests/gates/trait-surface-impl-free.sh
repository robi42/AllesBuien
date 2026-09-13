# shellcheck shell=bash
#
# A trait exactly at the limit, doc comments that say fn in prose, and an
# inherent impl with more methods than the limit allows. Only the trait's own
# surface counts; impl bodies are implementations, not interface.

gate="trait surface"
script="bin/check-fast"
outcome="green"
expect=()

mutate() {
  cat >core/src/violation_trait_at_limit.rs <<'RUST'
/// Four methods: every fn below is part of the surface.
pub trait Everything {
    /// The first fn.
    fn a(&self);
    fn b(&self);
    fn c(&self);
    fn d(&self);
}

pub struct Unit;

impl Unit {
    pub fn one(&self) {}
    pub fn two(&self) {}
    pub fn three(&self) {}
    pub fn four(&self) {}
    pub fn five(&self) {}
    pub fn six(&self) {}
}

impl Everything for Unit {
    fn a(&self) {}
    fn b(&self) {}
    fn c(&self) {}
    fn d(&self) {}
}
RUST
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
