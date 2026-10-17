# shellcheck shell=bash
#
# A thread-local is global state under another name: everything on its thread
# shares it, which only app/src/image_provider.rs is allowed to rely on.
# The static row may fire on the line inside it too; the expected substring
# thread_local! is on the macro's own line alone, so this fixture still fails
# if the thread_local! row stops biting.

gate="side-effect seams"
script="bin/check-fast"
outcome="red"
expect=("core/src/violation_thread_local.rs" "thread_local!")

mutate() {
  printf 'thread_local! {\n    static SEEN: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };\n}\n' >core/src/violation_thread_local.rs
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
