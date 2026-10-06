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
| `script` | `bin/check-fast`, `bin/check-qml-contract`, `bin/check-worktree-hook`, or `bin/test` |
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
which is git-ignored, gives it a throwaway git repository so the gates that
read `git ls-files` see the fixture's additions and deletions, and removes it on
exit. A killed run leaves `git status` clean and no worktree to prune. Signing
is disabled for the throwaway commit, so the suite never queues behind the SSH
agent.

Nothing here touches the repository either. In a linked worktree, which is
where the factory's builders commit, git hands every command it runs there
`GIT_DIR`, and a pre-commit hook `GIT_INDEX_FILE` as well, both naming the
worktree's repository; `git rebase -x bin/test` is enough to hand them to this
runner. A git command run in the scratch with them acts on that repository
instead: `git init` writes `core.bare = true` into the config every worktree
shares, which locks the main checkout out of every command that needs a work
tree, and the scratch's commits land on the worktree's branch. The main
checkout is no exception: git sets no `GIT_DIR` there, but a commit with `-a` or
of named paths hands its hook an absolute `GIT_INDEX_FILE`, and the scratch's
`git add` would write the commit's own index. A commit's `-c` options reach the
hook too, as `GIT_CONFIG_PARAMETERS`, and would run the scratch's commit with
them. So the runner reads the tree with those variables, since a commit with
`-a` or of named paths hands its hook an index only `GIT_INDEX_FILE` names, and
then clears git's local environment, as githooks(5) says to before running git
in another repository; the fixtures and gates run in the scratch inherit that.

`bin/check-worktree-hook`, the `worktree hook` gate, keeps it so. It builds a
decoy repository from a copy of the tree, with a linked worktree and a change
staged there, and runs the hook's `bin/check-fast` and this runner, on one
fixture whose `mutate` runs git, in that worktree with what `git commit -a`
hands the hook: `GIT_DIR`, an index of the commit's own that only
`GIT_INDEX_FILE` names, and `-c` options as `GIT_CONFIG_PARAMETERS`. The
worktree's own index lists a file the working tree no longer has, so a script
that reads it instead of the commit's fails on that file. A script that fails,
changes any file under the decoy's `.git`, or makes a commit with those options
fails the gate, which names the files and what `core.bare` became; a commit made
elsewhere with the same variables has to change the decoy and run the hook the
options name first, or an untouched decoy would prove nothing. Its five
fixtures put a defect into each script in turn: the runner without the line
that clears the variables, clearing them before it reads this repository, or
keeping the `-c` options, and `bin/check-fast` reading this repository without
the variables, or running `git init` in a scratch directory of its own.

Gates are run, never reimplemented. `bin/check-fast` and
`bin/check-qml-contract` are whole scripts and run as themselves; a gate that
lives inside `bin/test` is sliced out of that file by name and executed as
written, preamble included. A fixture that carried its own copy of a gate's
commands would keep passing long after the real gate changed underneath it.

## Coverage

Twenty-two of the twenty-six gates are covered. The four that are not all need a
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

`release version` holds the nine files that carry the version to
`app/Cargo.toml`: the other four Cargo manifests, the five workspace entries in
`Cargo.lock`, `pkgver=` in the PKGBUILD, the top-level `"version"` in
`manifest.json`, and every package filename in the README. `release.yml`
compares only three of them, and only at tag time, after the README has already
shipped a filename nobody can download. A surface that yields no version is
reported as `none`, so deleting a `pkgver=` line is refused rather than skipped.
Its two fixtures move `pkgver=` and the manifest's version, two of the three
surfaces no Cargo command ever rewrites.

`omarchy plugin` is the part of `omarchy-plugin-validate` that tracked text can
answer: no symlink anywhere in the tree, a tracked `manifest.json` at
`schemaVersion` 1 with its five required keys, an id in the validator's
character set and outside `omarchy.*`, every entry point relative, free of
`..`, and tracked, a non-empty `kinds` with an entry point for each kind in the
validator's table, and a `barWidget.defaultSection` of `left`, `center`, or
`right` when one is given. It is a belt, not the validator: it reads a
pretty-printed manifest line by line and says nothing about JSON validity or
value types, and CI's bare Arch container has neither `jq` nor an Omarchy
desktop to run the real thing. README.md's *Plugin checks* run the validator by
hand on a clean clone before every tag, which is still what proves the plugin
installs. Its fixtures add a symlink at the root and delete the one entry
point.

`theme-owned colours` reads `plugin/*.qml` as well as `app/qml/*.qml`, because
the bar widget has to follow the shell's theme as the window follows Omarchy's,
and it has a hex fixture of its own in `plugin/` so the widened file list has
been watched to fail. Deleting the entry point, above, also shows it reads an
empty `plugin/` as nothing rather than as a glob grep cannot open. Its file
list comes from the index, which can name a file the working tree has lost, so
a file it cannot read is an offender rather than a clean scan; another of its
fixtures leaves exactly that behind, using an assume-unchanged entry so the
runner's `git add --all` does not stage the deletion away.

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
fifty it runs, each of which fails loudly if its gate stops biting. One
of those fifty aims at the runner rather than at a gate: the slicer
checks a gate exists by substring and cuts its block by whole line, so a
reformatted header satisfies the first and cuts nothing under the second, and
the empty block that results goes green where the fixture asked for red. That
is caught — the verdict comparison sees it — but it is caught as "the gate
stopped biting" when the gate never ran, so the slicer now says so itself.
