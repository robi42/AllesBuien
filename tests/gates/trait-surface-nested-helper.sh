# shellcheck shell=bash
#
# A trait at the limit whose default method declares a helper fn in its body.
# The helper is an implementation detail of one method, not a fifth thing an
# implementor has to provide, so counting every fn line inside the braces would
# refuse a trait that is exactly as wide as the budget allows.

gate="trait surface"
script="bin/check-fast"
outcome="green"
expect=()

mutate() {
  cat >core/src/violation_trait_helper.rs <<'RUST'
pub trait Everything {
    fn a(&self);
    fn b(&self);
    fn c(&self);
    fn d(&self) {
        fn helper() {}
        helper();
    }
}
RUST
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
