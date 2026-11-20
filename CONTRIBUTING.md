# Contributing to AllesBuien

Bugs and ideas are welcome as [GitHub issues](https://github.com/robi42/AllesBuien/issues), and pull requests are welcome too. A security problem goes privately, as [SECURITY.md](SECURITY.md) describes, never into an issue.

## Before you start

Open an issue first for anything bigger than a fix, so that the direction is agreed before the work is done. A few directions are ruled out, and a pull request that takes one will not be merged, however well it is done:

- another weather provider in place of Buienradar, or beside it;
- radar beyond the Netherlands: the app draws Buienradar's Dutch radar and nothing else;
- finding your location from your IP address, or geocoding a place name;
- a location picker in the app: `omarchy-weather-location` already sets the desktop's location, and a second place to set it is what the app avoids;
- a bar widget that does more than open, focus, install, or update the app: the Omarchy weather pill beside it already shows the weather.

README's [Location](README.md#location) section explains the location choices, and AGENTS.md's [Anti-Patterns](AGENTS.md#anti-patterns) list the rest, each with its reason.

## Setting up

You need Arch Linux or Omarchy on x86_64, with `qt6-base` and `qt6-declarative`, which on Arch include the development headers. Then, from the root of a clone:

```bash
mise install
```

That installs every tool `mise.toml` pins, from Rust 1.99.1 with its components to lefthook, as README's [Development](README.md#development) section lists them. It also trusts the checkout's `mise.toml` without asking, and installs the pre-commit hook that runs `cargo fmt --all --check` and `bin/check-fast` before every commit. If you only want to build, rustup alone is enough: `rust-toolchain.toml` selects 1.99.1.

Not on Arch? Replay the quality job of [`.github/workflows/ci.yml`](.github/workflows/ci.yml) in an `archlinux:base-devel` container: its environment, then its steps in order, from the pacman line that installs Qt and rustup to `bin/test`.

## The fast loop

```bash
bin/check-fast                     # the text-only gates, in about a quarter of a second
cargo test -p allesbuien-core      # the platform-independent core, which needs no Qt
QT_QPA_PLATFORM=offscreen cargo test -p allesbuien --test <file>   # one GUI harness from app/tests/
just run                           # the app, from this checkout
```

The first build of the `allesbuien` crate takes a while, because CXX-Qt generates and compiles C++. Later builds are incremental, except that `cargo clippy` and `cargo test` invalidate each other's CXX-Qt build, so alternating between them rebuilds it each time.

## The full gate

`bin/test` is the whole gate, and the one CI runs. It takes over twenty minutes from cold and leaves a few gigabytes under `target/`. CI runs it on every pull request, so you can leave the full run to CI and keep to the fast loop locally. A new push to a pull request cancels the run still going for the push before it. A pull request from outside the repository runs CI only once the maintainer has approved the run, so expect a wait before the first result.

## What the gates hold you to

Each of these fails `bin/check-fast` or `bin/test`; [AGENTS.md](AGENTS.md#conventions) gives the reasons.

- `allesbuien-core` depends on no Qt, CXX, or CXX-Qt crate.
- Processes, environment reads, HTTP clients, and the Tokio runtime appear only in the files the side-effect seams table in `bin/check-fast` names, and nothing but the image provider's frame store holds process-global mutable state.
- No trait has more than four methods.
- No tracked `.rs` file has more than 250 lines that are neither blank nor comments, no function more than 50 lines, and no block nests more than six deep. Split along a responsibility, never by squeezing the formatting.
- rustfmt, Clippy with its pedantic lints and warnings denied, ShellCheck, actionlint, and qmllint all pass.
- Each file directly under `app/tests/` holds exactly one `#[test]`, because each GUI test needs a process of its own.
- QML takes every colour from the theme; `Theme.qml`'s red error fallback is the one literal.
- Every `extern "RustQt"` block carries `#[auto_cxx_name]`, and the generated types answer every name `app/qml` asks for.
- The window and README keep `Buienradar.nl`, linked to `https://www.buienradar.nl`.
- Every action a workflow uses is pinned to a full commit SHA, with its release tag in a comment.
- The nine files that carry the version agree with `app/Cargo.toml`, and `cxx` and `cxx-gen` stay at the same patch version.
- Only the images on `bin/check-fast`'s allowlist are tracked, and the tree holds no symlink.
- The workspace tests pass in the local time zone and again in UTC, `Asia/Kolkata`, `Pacific/Auckland`, and `America/New_York`, with at least 90% line coverage of `allesbuien-core`.
- A new gate ships with a fixture in `tests/gates/` that it is watched refusing.

Two rules no gate can check are yours to keep: tests use fakes, never a live endpoint, and never change the process environment; and a GUI harness waits with `pump_until`, never for a fixed number of event-loop passes.

## Pull requests

- Give the pull request a [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) title in the style `git log` shows, such as `fix(radar): …`. Pull requests are squash-merged, and the title becomes the subject of the one commit that lands on `main`.
- Add a line under Unreleased in `CHANGELOG.md` for anything a user of the latest release would notice. A fix to something no release has shipped needs none.
- Changes to the paths `.factory/protected` lists, the gate scripts, the workflows, the justfile, and mise's and lefthook's config among them, are welcome, though each such diff is read line by line before it lands: name them in the description, and say why.
- Work done with an AI assistant is welcome, and held to the same gate and the same review as any other.
- Keep at most two pull requests open at a time, drafts included.

The [pull request template](.github/pull_request_template.md) asks for the same things.

## Licence and attribution

AllesBuien is [MIT-licensed](LICENSE), and so is what you contribute: opening a pull request means you agree to license it under the same terms. Keep the Buienradar attribution as it is: Buienradar's free data may be used only with `Buienradar.nl` shown and linked, in the app and in README.

## Maintainer tooling you can ignore

The maintainer runs an agent pipeline of their own, the software factory. It is not part of this repository, and you need none of it:

- `.beads/` configures [beads](https://github.com/gastownhall/beads), the tracker that pipeline works from. Its database lives in a private repository, robi42/AllesBuien-beads, so a clone carries only this configuration. File your work as GitHub issues instead.
- `.factory/gate`, which runs `bin/test`, and `.factory/protected` are the factory's only files here. For you, `bin/test` is what counts.
- AGENTS.md is written for agents, and an agent of your own should skip its beads and factory notes.

The history carries the pipeline's marks. An `hq-` or `ab-` prefix in a commit or pull request title is a bead's ID; `hq-` became `ab-` on 2026-10-03. `fy` is the maintainer's agent factory, and its worktrees are the `factory/*` branches. `Co-authored-by` and `Claude-Session` trailers record the agents that worked on a commit.

## Reviewing outside pull requests

This last part is the maintainer's checklist; contributors can stop reading here.

An outside pull request's branch is untrusted code, and the maintainer's desktop is where it would do the most harm. That machine holds the SSH key that signs release tags, sudo, and a gh login that can push to `main`, rewrite workflows, and move tags, and that git uses over HTTPS too; the owner's bypass of the repository's branch and tag rules lets that login past them as well. The branch's code runs there as soon as one of the usual commands touches it:

- `mise install` trusts the branch's `mise.toml` and runs its postinstall;
- the pre-commit hook runs the branch's `lefthook.yml` jobs, `./bin/check-fast` among them;
- `bin/test`, `cargo test`, and `just run` build it, which runs its build script and proc macros, and the first two run its tests;
- `just install` runs sudo.

CI is the safe first run: a pull request's workflows are the branch's own files, but they get a read-only token and no secrets. So:

- Let CI run first, and approve a fork's workflow runs only after reading the diff.
- Before checking the branch out, read its changes to the paths `.factory/protected` lists, `app/build.rs`, every `Cargo.toml` and `Cargo.lock` (new crates, git sources), `.cargo/config.toml`, `rust-toolchain.toml`, `pkgbuild/PKGBUILD`, `plugin/widget.sh`, `tests/widget/`'s runner and fakes and its QML check with the fake modules beside it, and every `.gitattributes`, which decides how git writes the rest when you check the branch out.
- Check it out in a throwaway worktree or a container.
- Never run `mise install`, or commit with hooks, on a branch whose `mise.toml` and `lefthook.yml` you have not read.
