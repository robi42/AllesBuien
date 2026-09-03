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

Fifteen of the nineteen gates are covered. The four that are not all need a
compiled workspace, and a cold CXX-Qt build on this host is over twenty minutes:

| gate | why it is out of scope |
|------|------------------------|
| clippy | needs a full build per fixture, and `clippy` and `test` invalidate each other's cxx-qt fingerprints, so one fixture costs two rebuilds |
| workspace tests | same, plus a fixture would have to break a test to prove the runner reports it, which `cargo test` demonstrates on every red run anyway |
| timezone matrix | runs the whole suite four more times; the property it guards is proven by the suite it re-runs, not by the loop |
| core coverage | an instrumented rebuild, and the threshold is a number `cargo llvm-cov` enforces rather than a check written here |

The trade is deliberate. All four are stock invocations of tools whose failure
behaviour is not in question, whereas every gate covered here is either
hand-written shell — where the failure mode is a quoting mistake nobody sees —
or a check whose message has to name an offender to be worth reading. The two
that most needed proving, core purity and CXX-Qt name binding, are hand-written
shell that has already been wrong once each.

`gate violations` does not have a fixture of its own; it is proven by the
twenty-three it runs, each of which fails loudly if its gate stops biting.
