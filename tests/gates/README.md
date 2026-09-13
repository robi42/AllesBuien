# Gate violation fixtures

Each file here is one deliberately broken input, aimed at one gate, together
with the substring the resulting failure has to contain. `bin/check-gates`
overlays each onto a throwaway copy of the tree and asserts the gate goes red
for the stated reason, and that the same gate is green on the tree untouched.
`bin/test` runs it last, as `==> gate: gate violations`.

The reason this exists is measured rather than theoretical. `bin/test`'s
core-purity guard was `cargo tree | grep -qi cxx && exit 1 || true`, and under
`pipefail` that pipeline reports success precisely when it should fail. It sat
green and useless until somebody measured it, because a gate that cannot fail
looks exactly like a gate that is passing. Four gates had been watched to fail
at some point, each through a hand mutation in a scratch worktree that was then
thrown away; the other thirteen had never been shown to fail at all.

## Writing a fixture

A fixture is sourced by the runner and declares five things:

| name | meaning |
|------|---------|
| `gate` | the gate's name, exactly as its script echoes it — the runner refuses a fixture naming a gate that no longer exists |
| `script` | `bin/check-fast`, `bin/check-qml-contract`, or `bin/test` |
| `outcome` | `red` if the gate must refuse this input, `green` if it must tolerate it |
| `expect` | substrings the failure output must contain; a `red` fixture without one is rejected |
| `mutate` | a function run with the scratch copy as its working directory |

Write `gate="…"` in column one. `bin/check-fast`'s `gate coverage` gate reads
those declarations to work out which gates have a fixture at all, and it reads
them by shape rather than by search — exactly as the slicer reads a gate header
— so an indented or reworded copy is prose to both of them.

New fixtures keep the `shell=bash` declaration on their first line, which
the runner's contract needs, but silence no ShellCheck warning. The runner
reads `gate`, `script`, `outcome`, and `expect` after sourcing the file, which
ShellCheck cannot see, so each ends with one line that uses them instead:
`: "$gate" "$script" "$outcome" "${expect[@]}"`, under a comment saying why.
The older fixtures keep the SC2034 directive they were written with.

`args` optionally passes arguments to the gate script, which is how the two
contract fixtures point `bin/check-qml-contract` at a deliberately broken copy
of `plugin.qmltypes` rather than at the real one.

`mutate` may append to `expect`, which is how a fixture that picks its target
out of the tree — the first bridge carrying `#[auto_cxx_name]`, the first member
the QML reads — still asserts the offender by name. Prefer that over a hardcoded
path: a fixture pinned to a file that gets renamed fails for the wrong reason.

Everything runs under `set -euo pipefail`, so a `mutate` that ends in `| head -1`
will abort on SIGPIPE. Use `awk 'NR == 1'`, which reads to the end. That trap is
the same one that disabled the core-purity gate, and it would be a poor joke to
reintroduce it here.

The `green` fixtures are not filler. A gate wide enough to refuse a legitimate
input gets weakened by the first person it blocks, so the colour gate is pinned
against a controller binding, the size budget against a 751-line file that is
750 comments and 250 statements, and the name-binding gate against a module doc
that quotes `extern "RustQt"` verbatim — the false positive a gate that counted
occurrences instead of reading structure would produce on a clean tree.

## Isolation

Nothing here touches the real working tree. The runner copies the tracked and
untracked-but-not-ignored files into a `mktemp` directory under `target/`,
which is git-ignored, gives it a throwaway git repository so the six gates that
read `git ls-files` see the fixture's additions and deletions, and removes it on
exit. A killed run leaves `git status` clean and no worktree to prune. Signing
is disabled for the throwaway commit, so the suite never queues behind the SSH
agent.

Gates are run, never reimplemented. `bin/check-fast` and
`bin/check-qml-contract` are whole scripts and run as themselves; a gate that
lives inside `bin/test` is sliced out of that file by name and executed as
written, preamble included. A fixture that carried its own copy of a gate's
commands would keep passing long after the real gate changed underneath it.

## Coverage

Nineteen of the twenty-three gates are covered. The four that are not all need a
compiled workspace, and a cold CXX-Qt build on this host is over twenty minutes:

| gate | why it is out of scope |
|------|------------------------|
| clippy | needs a full build per fixture, and `clippy` and `test` invalidate each other's cxx-qt fingerprints, so one fixture costs two rebuilds |
| workspace tests | same, plus a fixture would have to break a test to prove the runner reports it, which `cargo test` demonstrates on every red run anyway |
| timezone matrix | runs the whole suite four more times; the property it guards is proven by the suite it re-runs, not by the loop |
| core coverage | an instrumented rebuild, and the threshold is a number `cargo llvm-cov` enforces rather than a check written here |

Two of the covered gates are design checks, and each is a proxy rather than a
proof. `side-effect seams` names the constructors for processes, environment
reads, HTTP clients, and the Tokio runtime, plus the ordinary spellings of
process-global mutable state, and allows each in exactly one home; it reads
only files under a `src/` directory and skips `#[cfg(test)]` blocks, because
build scripts must read the environment and test fakes build failing clients on
purpose. `trait surface` refuses a trait with more than four methods, required
and default alike, counting only the methods one level inside the trait so a
helper declared in a default body is not surface. Each constructor row has its
own red fixture and the static row has two, at column zero and inside an inline
module, so a broken row cannot hide behind a working one. Nesting depth is bounded by clippy's
`excessive_nesting`, which rides on the excused `clippy` gate. The open-closed
and Liskov principles have no fixture and no gate: nothing in the text of a
Rust file says whether an extension needed an edit or a subtype broke a
contract, and a gate that guessed would be a decoration.

`cxx family lockstep` reads `Cargo.lock` and refuses a `cxx` and a `cxx-gen`
whose patch numbers differ, or either crate missing or locked twice. cxx names
every bridge symbol after its patch and cxx-gen writes their C++ half, so the
mismatch a grouped Dependabot bump leaves behind compiles and then fails to link
eighteen minutes into `bin/test`. Its fixture reads the current versions out of
the lock rather than pinning them. The rest of the cxx family is not compared:
cxx's own manifest pins `cxx-build` and the `cxxbridge-*` crates exactly, so
cargo already holds them in step.

The trade is deliberate. All four are stock invocations of tools whose failure
behaviour is not in question, whereas every gate covered here is either
hand-written shell — where the failure mode is a quoting mistake nobody sees —
or a check whose message has to name an offender to be worth reading. The two
that most needed proving, core purity and CXX-Qt name binding, are hand-written
shell that has already been wrong once each.

That table has a machine-readable half. `tests/gates/uncovered` names the same
four gates, one per line, spelled as their script echoes them and each carrying
its reason as a comment. `bin/check-fast`'s `gate coverage` gate reads it and
refuses any declared gate that has neither a fixture nor an entry there, so
"covered or excused on purpose" stops being a claim this file makes and becomes
one the tree enforces. The list is read in both directions as well: a name that
no longer matches a real gate, or one a fixture has since covered, is refused
too, because an allowlist that only grows fills with entries asserting nothing.
It carries no `.sh` extension on purpose, so `bin/test`'s
`shellcheck bin/* tests/gates/*.sh` leaves it alone.

That arithmetic deliberately does not live here. This runner reports how many
gates its own fixtures named, which is silent by construction about a gate no
fixture named — the summary keeps printing the same totals and nothing goes red.
It could not live here either way: the selector argument makes this script's
gate list legitimately partial, and a fixture aimed at a gate inside this file
would nest the whole suite in itself. `bin/check-fast` is where `lefthook` runs
it, on the commit that would introduce the unproven gate.

`gate violations` does not have a fixture of its own; it is proven by the
thirty-eight it runs, each of which fails loudly if its gate stops biting. One
of those thirty-eight aims at the runner rather than at a gate: the slicer
checks a gate exists by substring and cuts its block by whole line, so a
reformatted header satisfies the first and cuts nothing under the second, and
the empty block that results goes green where the fixture asked for red. That
is caught — the verdict comparison sees it — but it is caught as "the gate
stopped biting" when the gate never ran, so the slicer now says so itself.
