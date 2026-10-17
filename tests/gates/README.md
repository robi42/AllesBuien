# Gate violation fixtures

Each file here is one deliberate input aimed at one gate: a broken one, with
the substrings its failure has to contain, or a legitimate one the gate has to
tolerate. `bin/check-gates` overlays each onto a throwaway copy of the tree and
asserts the gate goes red for the stated reason, or stays green, and that the
same gate is green on the tree untouched.
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

New fixtures keep the `# shellcheck shell=bash` directive on their first line,
which ShellCheck needs for a file with no shebang, but silence no ShellCheck
warning. The runner reads `gate`, `script`, `outcome`, and `expect` after
sourcing the file, which ShellCheck cannot see, so each ends with one line that
uses them instead: `: "$gate" "$script" "$outcome" "${expect[@]}"`, under a
comment saying why. The older fixtures keep the SC2034 directive they were
written with.

`args` optionally passes arguments to the gate script, which is how the three
contract fixtures point `bin/check-qml-contract` at an altered copy of
`plugin.qmltypes` rather than at the real one.

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
against a controller binding, the size budget against a 751-line file whose 250
statements sit among 501 comment and blank lines, and the name-binding gate
against a module doc that quotes `extern "RustQt"` verbatim — the false positive
a gate that counted occurrences instead of reading structure would produce on a
clean tree.

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
worktree's repository; `git rebase -x bin/test` hands this runner `GIT_DIR`
alone, which is enough. A git command run in the scratch with them acts on that
repository instead: `git init` writes `core.bare = true` into the config every
worktree shares, which locks the main checkout out of every command that needs
a work tree, and the scratch's commits land on the worktree's branch. The main
checkout is no exception: git sets no `GIT_DIR` there, but a commit with `-a` or
of named paths hands its hook an absolute `GIT_INDEX_FILE`, and the scratch's
`git add` would write the scratch's files into the commit's own index. The
commit then fails on an object only the scratch holds or, when the repository
already holds every object it names, goes through with what the scratch
staged: a commit of `README.md` whose hook ran this runner on
`omarchy-plugin-entry-point-missing` deleted `plugin/BarWidget.qml` as well.
Options given as `git -c` reach whatever git runs, as `GIT_CONFIG_PARAMETERS`,
and git in the scratch would run with them. Which of the three a run hands
over depends on how git ran it: `git rebase -x` hands `GIT_DIR` alone in a
linked worktree and only the `-c` options in the main checkout, and a commit
made without `-c` options leaves `GIT_CONFIG_PARAMETERS` unset. So the runner
reads the tree with those variables, since a commit with `-a` or of named paths
hands its hook an index only `GIT_INDEX_FILE` names, and then clears git's
local environment, whichever of them is set, as githooks(5) says to before
running git in another repository; the fixtures and gates run in the scratch
inherit that. The scratch's commit runs no hook: clearing git's local
environment cannot stop one the global config names, and a global lefthook
setup would run the scratch's own `lefthook.yml` there on every run. It looks
for its hooks in `/dev/null`, given as `-c core.hooksPath=/dev/null`, where
there are none, in place of a hooks directory: the one a `core.hooksPath` in
the global config names, or the scratch's own, where an `init.templateDir`
there puts hooks at `git init`. That does not reach a hook a config defines by
name, as `hook.<name>.command`, which git 2.55.0 runs for each event its
`hook.<name>.event` lists whatever `core.hooksPath` says: one the global config
defines, or one in the scratch's own config, into which `git init` copies the
`config` an `init.templateDir` holds. So the commit also sets
`hook.<event>.enabled=false` for each event it runs: `pre-commit`,
`prepare-commit-msg`, `commit-msg`, `post-commit`, `reference-transaction` and
`post-index-change`. That setting leaves a hooks directory's hooks running, so
the commit needs both.
The copy also leaves out what git lists as a whole directory: a repository of
its own, such as a linked worktree checked out under this one, whose `.git` file
names this repository, so the scratch's `git status` would enter it and rewrite
its index.

`bin/check-worktree-hook`, the `worktree hook` gate, keeps it so. It builds a
decoy repository from a copy of the tree, with a linked worktree, and in each
checkout a change staged and another linked worktree checked out inside it. It
runs the hook's `bin/check-fast`, this runner, on one fixture whose `mutate`
runs git, and itself four ways, so that each of the three variables is the
only one set in one of them. First as a commit runs its hook: in the worktree
with what `git commit -a`, given `-c` options, hands it there — `GIT_DIR`, an
index of the commit's own that only `GIT_INDEX_FILE` names, and the options as
`GIT_CONFIG_PARAMETERS` — and in the main checkout with what `git commit` with
paths, given none, hands it there — the commit's index by its absolute path,
`.git/next-index-1.lock`, and nothing else. Each checkout's own index lists a
file the working tree no longer has, so a script that reads it instead of the
commit's fails on that file. Then as `git rebase -x` runs a command, once each
checkout's own index has become a copy of its commit's: in the worktree with
`GIT_DIR` alone, and in the main checkout with the options alone. Clearing the
variables only when one of them is set, or either of two, meets a run that
hands over the others alone. The options name a hooks path with a hook under
every name githooks(5) lists: `git init` runs `reference-transaction` there, and
`git add` and `git status` `post-index-change`, so a script that runs git with
them leaves a mark, though its commits look for their hooks in `/dev/null`,
which outranks the options. A script that fails, changes any file under the
decoy's `.git`, or runs git with those options fails the gate, which names the
files and what `core.bare` became, or the hooks that ran. Everything runs under
a global config of the gate's own, in place of the machine's, whose pre-commit
and prepare-commit-msg hooks fail any commit that runs them. Each is there
twice, in the hooks directory the config names and defined by name, so a
script whose commit runs the hooks the global config names fails too, and so
does one that skips them with `--no-verify` instead of naming `/dev/null`: that
skips `pre-commit` and `commit-msg`, but not `prepare-commit-msg`,
`post-commit`, `reference-transaction` or `post-index-change`. So does one that
names `/dev/null` and leaves the hooks defined by name on for either event. Its
`init.templateDir` puts both kinds into every repository `git init` makes, the
hooks into its hooks directory and those defined by name into its config, so
one whose commit reads no global config, given `GIT_CONFIG_GLOBAL=/dev/null`
instead of either setting, still runs them and fails. A commit made elsewhere
with the worktree's hook variables has to change the decoy and run the hooks
the options name, commits made elsewhere without them have to fail on each kind
of hook the global config has, and so do commits that read no global config,
on each kind `git init` copied, and an add made elsewhere with the main
checkout's has to change the decoy's copy of the commit's index, before any
script runs, or an untouched decoy, or a commit that passes, would prove
nothing; the runs as `git rebase -x` runs a command hand over the same
`GIT_DIR` and the same options. The gate runs itself because `bin/test` runs
it, under whatever variables `bin/test` was handed, and it builds the decoy
with git: the copy, passed `nested`, builds a decoy of its own, makes every git
call the gate makes, and stops before it would run the scripts. Its seventeen
fixtures put a defect into each script in turn: the runner without the line
that clears the variables; clearing them only when `GIT_DIR` is set, only when
`GIT_INDEX_FILE` is, only when either of those two is, or only when `GIT_DIR`
or `GIT_CONFIG_PARAMETERS` is; clearing them before it reads this repository,
keeping the `-c` options, or copying the nested worktree; committing with the
hooks the global config names, skipping them with `--no-verify`, or reading no
global config instead, which leaves the hooks `git init` copied from it;
committing with the hooks the global config defines by name, turning them off
for `pre-commit` alone, or reading no global config instead, which leaves
those defined in the config `git init` copied; `bin/check-fast` reading this
repository without the variables, or running `git init` in a scratch directory
of its own; and the gate itself without the line that clears them.
The last of the four keyed ones goes red only while the gate runs the main
checkout's hook without `-c` options, so it holds the gate to that.

Gates are run, never reimplemented. `bin/check-fast` and
`bin/check-qml-contract` are whole scripts and run as themselves; a gate that
lives inside `bin/test` is sliced out of that file by name and executed as
written, preamble included. A fixture that carried its own copy of a gate's
commands would keep passing long after the real gate changed underneath it.

## Coverage

Twenty-three of the twenty-seven gates are covered. The four that are not all
need a compiled workspace, and a cold CXX-Qt build takes over twenty minutes:

| gate | why it is out of scope |
|------|------------------------|
| clippy | needs a full build per fixture, and `clippy` and `test` invalidate each other's cxx-qt fingerprints, so one fixture costs two rebuilds |
| workspace tests | same, plus a fixture would have to break a test to prove the runner reports it, which `cargo test` demonstrates on every red run anyway |
| timezone matrix | runs the whole suite four more times; the property it guards is proven by the suite it re-runs, not by the loop |
| core coverage | an instrumented rebuild, and the threshold is a number `cargo llvm-cov` enforces rather than a check written here |

Two of the covered gates are design checks, and each is a proxy rather than a
proof. `side-effect seams` names the constructors for processes, environment
reads, HTTP clients, and the Tokio runtime, plus the ordinary spellings of
process-global mutable state, and allows each only in the files its table names;
it reads only files under a `src/` directory and skips `#[cfg(test)]` blocks,
because build scripts must read the environment and test fakes build failing
clients on purpose. `trait surface` refuses a trait with more than four methods,
required and default alike, counting only the methods one level inside the trait
so a helper declared in a default body is not surface. Each constructor row has
its own red fixture; the static row has two, at column zero and inside an inline
module; and the `thread_local!` row has one, so a broken row cannot hide behind
a working one. Nesting depth is bounded by clippy's `excessive_nesting`, which
rides on the excused `clippy` gate. The open-closed and Liskov principles have
no fixture and no gate: nothing in the text of a Rust file says whether an
extension needed an edit or a subtype broke a contract, and a gate that guessed
would be a decoration.

`cxx family lockstep` reads `Cargo.lock` and refuses a `cxx` and a `cxx-gen`
whose patch numbers differ, or either crate missing or locked twice. cxx names
every bridge symbol after its patch and cxx-gen writes their C++ half, so the
mismatch a grouped Dependabot bump leaves behind compiles and then fails to link
eighteen minutes into `bin/test`. Its fixture reads the current versions out of
the lock rather than pinning them. The rest of the cxx family is not compared:
cxx's own manifest pins `cxx-build` and the `cxxbridge-*` crates exactly, so
cargo already holds them in step.

`release version` holds the nine files that carry the version to
`app/Cargo.toml`: the other four Cargo manifests, the five versioned workspace
entries in `Cargo.lock`, `pkgver=` in the PKGBUILD, the top-level `"version"` in
`manifest.json`, and every package filename in the README. `release.yml`
compares only three of them, and only at tag time, after the README has already
shipped a filename nobody can download. A surface that yields no version is
reported as `none`, so deleting a `pkgver=` line is refused rather than skipped.
Its two fixtures move `pkgver=` and the manifest's version, two of the three
surfaces no Cargo command ever rewrites.

`lefthook pin` holds `lefthook.yml`'s `min_version` to the lefthook
`mise.toml` pins. Lower, and an older lefthook first on PATH runs the hook
again; higher, and the pinned one refuses every commit. Its fixture moves the
pin and leaves `min_version` behind.

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

That arithmetic deliberately does not live in the runner. `bin/check-gates`
reports how many gates its own fixtures named, which is silent by construction
about a gate no fixture named — the summary keeps printing the same totals and
nothing goes red. It could not live there either way: the selector argument
makes the runner's gate list legitimately partial, and a fixture aimed at a gate
inside `bin/check-gates` would nest the whole suite in itself. `bin/check-fast`
is where `lefthook` runs it, on the commit that would introduce the unproven
gate.

`gate violations` does not have a fixture of its own; it is proven by the
sixty-five it runs, each of which fails loudly if its gate stops biting. One
of those sixty-five aims at the runner rather than at a gate: the slicer
checks a gate exists by substring and cuts its block by whole line, so a
reformatted header satisfies the first and cuts nothing under the second, and
the empty block that results goes green where the fixture asked for red. That
is caught — the verdict comparison sees it — but it is caught as "the gate
stopped biting" when the gate never ran, so the slicer now says so itself.
