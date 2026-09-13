# shellcheck shell=bash
#
# Five methods, one of them a default: a fake of this trait has to reason about
# all five to exercise one, and the default counts because an implementor still
# has to know whether to override it.

gate="trait surface"
script="bin/check-fast"
outcome="red"
expect=("core/src/violation_fat_trait.rs" "Everything" "5")

mutate() {
  cat >core/src/violation_fat_trait.rs <<'RUST'
pub trait Everything {
    fn a(&self);
    fn b(&self);
    fn c(&self);
    fn d(&self);
    fn e(&self) {
        self.a();
    }
}
RUST
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
