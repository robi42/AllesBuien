# AGENTS.md — AllesBuien

## Overview

AllesBuien is a native Omarchy weather application built as a Rust workspace with a Qt Quick/CXX-Qt desktop shell. It shows Buienradar current conditions, a five-day forecast, minute rain predictions, an animated Dutch radar, and best-effort rain-soon notifications while following the active Omarchy theme.

## Run and verify

Run from the repository root:

```bash
bash .factory/gate          # the definition of done: wraps bin/test
bin/check-fast              # bin/test's text-only first step, ~0.25s
bin/build                   # release build
bin/install                 # (cd pkgbuild && makepkg -f), then install
lefthook run pre-commit     # cargo fmt --check and bin/check-fast
```

`bash .factory/gate` is the definition of done and runs nothing but `bin/test`, which is the full workspace gate and the one CI runs — over twenty minutes from cold, because CXX-Qt regenerates C++. `bin/check-fast` is its text-only first step and what `lefthook` runs on every commit, so the hook can never hold a weaker opinion than the server. Tools are pinned in `mise.toml`, lefthook among them; run `mise install` first, which in the main checkout also writes the hook into `.git/hooks`, where every worktree shares it. In a linked worktree it leaves the hook alone, but `lefthook install` run by hand in one still rewrites it from that worktree's `lefthook.yml`, so run that in the main checkout only. A commit fails when the hook finds no lefthook, or one older than `mise.toml` pins: that is the machine's to fix, so say so in your report rather than skipping the hook with `--no-verify` or `LEFTHOOK=0`.

Do not change `.factory/gate` or any path listed in `.factory/protected` — a branch that touches one cannot be approved without a human reading the diff. `.factory/run/` is factory scratch and is git-ignored.

## Tasks and memory

- Work is filed as [beads](https://github.com/steveyegge/beads) in `.beads/`: `bd ready` lists what is actionable, `bd show <id>` reads one issue.
- The software factory claims and closes beads itself, so do not open, re-assign, or close them by hand during a run.
- Leave `bd remember "..."` notes only for what the code cannot tell a reader: a decision and its reason. Anything the tree already states belongs in a comment beside it, not in memory.

## Structure

```
AllesBuien/
├── core/                 Platform-independent weather, radar, notification, config, and theme logic
├── app/                  Qt/CXX-Qt application, controllers, QML UI, assets, and GUI integration tests
├── assets/gen-basemap/   Deterministic pure-Rust Natural Earth basemap generator and checker
├── plugin/               The Omarchy bar-widget launcher, loaded by omarchy-shell, not by the app
├── manifest.json         The Omarchy plugin manifest the marketplace and the shell both read
├── preview.png           The README image, and the preview the plugin marketplace shows
├── CHANGELOG.md          One section per release; the section is the tag annotation
├── bin/                  Canonical build, test, package-install, and gate entrypoints
├── tests/gates/          One known-bad input per gate, plus the allowlist of gates without one
├── pkgbuild/             Arch PKGBUILD, desktop entry, icon, and ignored package artifacts
├── .github/              SHA-pinned CI, release automation, and Dependabot configuration
├── .factory/             Software-factory gate and protected-path list; ignored run/ scratch
├── .beads/               Tracked issue-tracker config and history; the Dolt database is ignored
├── .qmllint.ini          Committed qmllint strictness, so a Qt bump is a reviewable diff
├── mise.toml             Rust 1.98.0, actionlint, shellcheck, and lefthook pins; installs the hook
├── CLAUDE.md             A regular file importing AGENTS.md, never a symlink: `omarchy plugin
│                         add` refuses a plugin folder that holds one anywhere
└── lefthook.yml          Pre-commit rustfmt and the text-only gates
```

## Where to Look

| Task | Location | Notes |
|------|----------|-------|
| Configuration | `core/src/config.rs` | Endpoints, notification thresholds, state directory, and the location setting |
| Location precedence and re-resolution | `core/src/location.rs`, `core/src/location_settings.rs` | Environment → Omarchy `weather.json` → Amsterdam-West default, re-askable at any time |
| Location provenance and radar coverage wording | `core/src/location_notice.rs` | Which source answered, and whether the Dutch radar reaches the place |
| Buienradar feed and raintext parsing | `core/src/weather.rs` | `FeedApi` boundary and forecast/current-condition models |
| Radar metadata, frames, and cache pruning | `core/src/radar.rs` | `RadarApi`, generation IDs, bounded run cache |
| Notification rules and adapter | `core/src/notify.rs` | Cooldown/event dedup behind `Notifier` |
| Omarchy theme parsing | `core/src/theme.rs` | Tolerant, path-resolving reads with `Unavailable` fallback state |
| Application construction and dependency injection | `app/src/bootstrap.rs`, `app/src/services.rs` | Qt ownership, Tokio runtime, controller seams |
| Weather bridge | `app/src/weather_controller.rs` | Async feed refresh and QML list model |
| Condition icons | `app/src/icon_delivery.rs`, `app/src/icon_cache.rs` | Disk pass, text, then per-icon downloads and re-publish |
| Radar bridge and image delivery | `app/src/radar_controller.rs`, `app/src/image_provider.rs` | Generation-safe playback and `image://radar` provider |
| What both bridges share | `app/src/refresh_job.rs`, `app/src/location_follow.rs`, `app/src/qt_values.rs` | The retry ladder and background refresh work, the location-moved check, and Duration/Option-to-Qt conversions |
| Theme bridge | `app/src/theme_controller.rs` | Replacement-aware 2-second polling |
| Desktop UI | `app/qml/Main.qml` | Radar, current conditions, forecast, inline statuses, attribution |
| GUI integration tests | `app/tests/` | Exactly one GUI test and one `QGuiApplication` per file/executable |
| Shared GUI test doubles | `app/testkit/` | Qt-free crate: the inert stubs several harnesses need, plus `pump_until` |
| Shared GUI boot, object-tree probes, and input | `app/harness/` | Qt-linked dev crate: `HarnessBuilder`, the `unsafe` rendered-tree probes, and keys and clicks entered through the window system |
| Basemap provenance | `assets/BASEMAP-LICENSE` | Pinned Natural Earth source and shipped-image SHA-256 |
| Packaging | `pkgbuild/PKGBUILD`, `pkgbuild/allesbuien.desktop` | Static package metadata and four installed payloads |
| Quality gates | `bin/test`, `bin/check-fast`, `bin/check-qml-contract` | `bin/test` is the whole list and CI runs it; the other two are its cheap and post-build halves |
| Proof that the gates can fail | `bin/check-gates`, `tests/gates/` | One known-bad input per gate, run in a scratch copy; `tests/gates/README.md` records what is covered and what is not |
| Gate scripts under a hook's git variables | `bin/check-worktree-hook` | Runs `bin/check-fast`, `bin/check-gates` and itself as git runs them from a commit's hook and from `git rebase -x`, in a linked worktree and in the main checkout, against a decoy repository, and refuses any change to it |
| The gate agents are held to | `.factory/gate`, `.factory/protected` | The wrapper around `bin/test`, and the paths whose diffs a human has to read |
| Omarchy plugin | `manifest.json`, `plugin/BarWidget.qml` | The bar launcher and the manifest the shell validates; `bin/check-fast`'s `omarchy plugin` gate is a text belt over the manifest, symlinks, and entry points, while the validator, qmllint, and the click-through stay manual |
| Release surfaces | `CHANGELOG.md`, README's *Releasing* section | The nine files carrying the version, and the manual plugin checks that precede a tag |

## Conventions

- **Core-purity seam:** `allesbuien-core` is platform-independent pure Rust with zero CXX, CXX-Qt, or Qt dependencies. Qt objects, timers, threads, image delivery, and process adapters belong in `app`.
- **Dependency injection:** Keep network and notification side effects behind the object-safe `FeedApi`, `RadarApi`, and `Notifier` traits. Pass services, paths, location, and timer intervals through the existing construction seams; do not introduce globals. `bin/check-fast` holds the line textually: `side-effect seams` allows each process, environment, HTTP-client, and runtime constructor, and process-global mutable state, in one named file, and `trait surface` refuses a trait wider than four methods; clippy's `excessive_nesting` (threshold 6, in `clippy.toml`) bounds block nesting. They are proxies for dependency inversion, interface segregation, and simplicity, not proofs.
- **Fakes over mocks:** Tests use small deterministic fake trait implementations with call counters or in-memory values; they never hit production networks or mutate process-global environment state. A fake several GUI harnesses need lives in `allesbuien-testkit`, so no harness has to silence a dead-code warning for the doubles it does not use; a fake one harness scripts stays beside the assertions it serves, as a private module under `app/tests/` if it is large enough to want a name.
- **Waiting on Qt:** GUI harnesses wait with `pump_until(condition, budget, attempt)` from `allesbuien-testkit`, never a hardcoded spin count. An iteration cap cannot tell falling out of the loop from succeeding in it, so an unreachable condition surfaces as whichever later assertion happens to read the unfinished state.
- **Qt threading:** HTTP, file reads, and notification dispatch run off the Qt thread; QObject changes return through the CXX-Qt thread queue.
- **Theme-watch re-arm:** Poll `~/.local/state/omarchy/current/theme` every 2000 ms, re-resolving the path on each changed read and tolerating disappearance. Omarchy replaces that directory by removing and renaming, which no inode-bound watcher survives.
- **Theme fallback:** Rust exposes `themeAvailable`; QML uses `SystemPalette` when unavailable. Theme colors otherwise come from the controller.
- **Location following:** Poll `~/.local/state/omarchy/settings/weather.json` every 2000 ms and never watch it — it is a desktop-wide setting that `omarchy-weather-location` rewrites and `--clear` unlinks. `LocationSettings` holds the inputs rather than the answer, and the weather and radar bridges each re-resolve and compare against the location they last adopted, so there is no shared cell to race and the order of the two `pollLocation()` calls carries no meaning. An `ALLESBUIEN_LAT`/`ALLESBUIEN_LON` pair is captured once at construction and outranks every later file change; `core/src/location_settings.rs`'s module doc argues all three points.
- **No location auto-detection, deliberately:** A missing `weather.json` falls back to Amsterdam-West, labelled as a default by `core/src/location_notice.rs`, and never to IP detection: this app draws Dutch radar and only Dutch radar, so a detected Berlin would be a confident-looking window over a permanently empty map. A name-only entry such as `--set Malibu` carries no coordinates and counts as no setting, because inventing them would be a second source of truth for a setting the desktop owns. README.md's *Location* section states the divergence from Omarchy's own widget in full.
- **Radar coverage:** A location outside `core/src/radar_projection`'s bounding box gets no marker and a stated notice, because a raster that paints its base and nothing else is indistinguishable from a failed frame request. The bounds are named once, in `radar_projection`; `location_notice` asks that module rather than restating them.
- **Radar delivery:** Metadata `times[]` determines frame order and cadence. Transparent frame PNGs are served without Rust-side composition through generation-keyed `image://radar/<generation>/<index>` URLs.
- **Icon delivery:** Condition icons never hold the weather text back. A refresh publishes the icons already on disk, then the text, then downloads the rest, re-publishing after each one. A provider URL is withheld as `""` until the store can serve it, so each re-publish is a change QML reacts to, and each slot's loading state (`heroIconLoading`, the `iconLoading` role) follows only its own key's download, counted per refresh, so a pending slot shows a skeleton and a failed one stays empty. The payload a refresh queues carries that refresh's generation and is dropped once a later refresh has been adopted, so a fetch that is overtaken cannot put its older payload back; the per-key counts are deliberately not tagged that way, because a download that has started has to be counted out again whoever started it.
- **Error UX:** Retain last-good data and expose one minimal inline status per view. Malformed or missing optional input must not panic.
- **GUI tests:** Every file directly under `app/tests/` contains exactly one GUI test and constructs exactly one `QGuiApplication`, giving each Qt application its own process. A second test in the same file meets a `QGuiApplication` that already exists and dies with a SIGSEGV that reads like a product bug.
- **Module size:** No tracked `.rs` file exceeds **250 pure lines** — non-blank, non-comment, measured by `grep -cvE '^[[:space:]]*(//|$)' <file>` and enforced by `bin/check-fast` — because past that a module has outgrown one reviewer's working memory and is doing more than one thing. One number governs the whole tree, GUI harnesses included; `bin/check-fast`'s comment records why they lost their exception. The number is a symptom and cohesion is the goal, so the only sanctioned fix is extracting a module with a name describing one responsibility, its own doc, and its own tests.
- **Toolchain and gates:** Use pinned Rust 1.98.0. `bin/test` is the source of truth for workspace verification, keeps core coverage at or above 90%, and runs `bin/check-fast` first and `bin/check-qml-contract` last for reasons its own comments give. A gate that cannot be watched to fail is a decoration, so `bin/check-gates` replays every fixture in `tests/gates/` on each run and asserts the gate goes red **with the message that names the offender**; ship a fixture with any new gate or excuse it by name in `tests/gates/uncovered`, which `bin/check-fast`'s `gate coverage` gate enforces in both directions, and read `tests/gates/README.md` for the count and the per-gate argument.
- **Git in another repository:** in a linked worktree, where the factory's builders commit, git hands every command it runs there — the pre-commit hook, an alias, `git rebase -x bin/test` — `GIT_DIR`, and the hook `GIT_INDEX_FILE` too, both naming the worktree's repository. A gate script that runs git anywhere else, as `bin/check-gates` does in its scratch, reads its own repository first, with them, and then unsets what `git rev-parse --local-env-vars` lists, as githooks(5) says, whichever of them is set: `git rebase -x` hands `GIT_DIR` alone in a linked worktree and only `-c` options, as `GIT_CONFIG_PARAMETERS`, in the main checkout, and a commit made without `-c` options leaves `GIT_CONFIG_PARAMETERS` unset, so clearing them only when one is set keeps the others. With them, its `git init` marks the worktree's repository bare, which locks the main checkout out of every command that needs a work tree, and its commits land on the worktree's branch. The main checkout is no exception: there git sets no `GIT_DIR`, but a commit with `-a` or of named paths hands the hook an absolute `GIT_INDEX_FILE`, the commit's own index, so a scratch `git add` writes the scratch's files into it, and the commit fails on an object only the scratch holds or, when the repository already holds every object it names, goes through with what the scratch staged: a commit of `README.md` whose hook ran the runner on `omarchy-plugin-entry-point-missing` deleted `plugin/BarWidget.qml` as well. `bin/check-worktree-hook` runs `bin/check-fast`, `bin/check-gates` and itself, since `bin/test` runs it too, four ways against a decoy, each of the three variables the only one set in one of them: in a linked worktree with the index and `-c` options `git commit -a` hands the hook there, in the main checkout with the index alone that `git commit` with paths hands it there, in a linked worktree with `GIT_DIR` alone, as `git rebase -x` hands it, and in the main checkout with the options alone, as `git rebase -x` run with `-c` options hands it there. It refuses a failure, any change to the decoy, and a commit made with those options; a new script that runs git elsewhere joins its list.
- **CXX-Qt name binding:** Every `extern "RustQt"` block carries `#[auto_cxx_name]`, with no exceptions, so multi-word members reach C++ in the camelCase QML reads them by. Without it nothing fails to compile and no test fails; the window simply comes up unthemed and unbound. `bin/check-fast` asserts the source shape and `bin/check-qml-contract` asserts the generated type registry still answers every member name `app/qml` asks for.
- **Clock-reading tests:** `core/src/radar_timeline.rs` and `core/src/datetime.rs` read the host clock on purpose, so banning `now()` would condemn the wrong lines. Tests have to hold in any zone instead, and `bin/test` re-runs the suite under UTC, `Asia/Kolkata`, `Pacific/Auckland`, and `America/New_York` to prove it — Kolkata's half-hour offset being what catches an assertion that pinned the minutes to `:00`.
- **Packaging:** `pkgver` is static. The canonical package build is exactly `(cd pkgbuild && makepkg -f)` as invoked by `bin/install` and automation.
- **Attribution:** Buienradar free data is for non-commercial use with attribution, and the terms name exactly two visible obligations: the words `Buienradar.nl` are shown, and they link to the root domain `https://www.buienradar.nl`. Keep both in the app and in the README. Any label wrapped around them is ours to word or drop; the GUI test asserts the domain text and the link, not a sentence.
- **Omarchy plugin is a launcher:** the widget runs `omarchy-launch-or-focus` when the app is installed and Omarchy's floating-terminal AUR install of `allesbuien-bin` when it is not, and nothing else; each click acts on a probe completed after the click, never on a remembered state, and the probe runs through the same `bash -lc` login shell `bar.run` uses, so both see `~/.local/bin`. No fetching, parsing, or theme logic in plugin QML — `bin/check-fast`'s colour gate reads `plugin/*.qml` too, so a literal colour is refused, but no gate enforces the fetching and parsing half. Its glyph is one static Nerd Font codepoint in the bar's font, never an image and never a weather symbol, because the Omarchy pill beside it already shows the condition and a second cloud or raindrop would read as a duplicate. Entry points must be tracked files and the tree may hold no symlink, because `omarchy plugin add` refuses one. It never names `omarchy.weather` in `clonedFrom`, so it coexists with the built-in pill instead of replacing it.
- **AUR:** `allesbuien-bin` repackages the GitHub release asset and is what the widget installs. Its recipe lives in the AUR repo, not here, because its checksum is of an artefact built from the tagged commit; README's *Releasing* section is the bump procedure, and `pkgbuild/PKGBUILD` stays the source-less local recipe.
- **Release surfaces:** nine files carry the version, listed by path in README's *Releasing* section; `release.yml` checks three of them against the tag, and `bin/check-fast`'s `release version` gate holds all nine to `app/Cargo.toml` on every commit. `CHANGELOG.md`'s section for a version is the text of its tag annotation.

## Anti-Patterns

- Do NOT add CXX, CXX-Qt, Qt, app-thread, or UI dependencies to `allesbuien-core`.
- Do NOT put domain or business logic in QML, controller C++, or the image-provider trampoline. The sole C++ carve-out is the existing construction/bridge surface; keep it minimal and logic-free.
- Do NOT block the Qt thread on HTTP, filesystem work, notification dispatch, or Tokio futures.
- Do NOT construct controllers outside the existing library bootstrap/injection seams or introduce global service state.
- Do NOT use mocks when a trait fake can express the scenario; do not call live endpoints from unit tests or mutate process-global environment variables in tests.
- Do NOT place multiple GUI tests or multiple `QGuiApplication` instances in one integration-test file/executable.
- Do NOT decode or compose radar/basemap images in Rust, hardcode radar cadence, retain stale generations, or allow an unbounded radar cache.
- Do NOT use a file/inode watcher for the Omarchy theme directory or the Omarchy weather settings file, fabricate fallback palette values, hard-fail on missing theme keys, or add custom C++ watcher bindings.
- Do NOT add IP geolocation, forward or reverse geocoding, or an in-app location picker. The desktop already ships `omarchy-weather-location`, and a second way to set a desktop-wide setting is the thing to avoid; the divergence from Omarchy's IP auto-detection is a recorded decision, not an omission.
- Do NOT hardcode colors in QML; bind controller theme properties or `SystemPalette` fallback values.
- Do NOT add toast/retry frameworks or notification spam; keep one inline error status per view and preserve cooldown/event dedup.
- Do NOT replace Buienradar with wttr.in/Open-Meteo, expand beyond Dutch radar, or remove the required attribution: the visible `Buienradar.nl` and its link to `https://www.buienradar.nl` both have to stay.
- Do NOT copy the donor `map-nl-*` image, any unlicensed basemap, or OSM tile-server imagery. Basemaps come only from the pinned Natural Earth public-domain sources through `assets/gen-basemap`.
- Do NOT write to `/home/robi42/Projects/buienradar`, add AllesBuien to the parent `/home/robi42/Projects` repository, or modify `/usr/share/omarchy`.
- Do NOT turn the app into a Quickshell widget; the bar widget under `plugin/` launches, installs, or focuses and must stay that thin. Do NOT touch built-in `omarchy.weather` or add a server/deploy/Docker surface. Publishing `allesbuien-bin` to the AUR is a recorded decision of 2026-09-18, not a violation of this line.
- Do NOT create `.SRCINFO`, an AUR remote, a dynamic `pkgver()` function, network-backed sources, or alternative package-build invocations **in `pkgbuild/`**; the AUR clone outside this tree is the one place those belong.
- Do NOT run `makepkg -s`, `makepkg -i`, `makepkg -si`, or `makepkg -fsi`; only `(cd pkgbuild && makepkg -f)` is sanctioned.
- Do NOT invoke bare `sudo` or any command that can wait for a password. Live install mode is user-scope Mode B unless the recorded decision is explicitly changed by its capability probe. This governs agents and `bin/install`. The widget's click-to-install and the README's `yay`/`sudo pacman` lines are commands a person runs at their own terminal, so an agent exercising the widget swaps the install string for an inert echo in its throwaway copy rather than clicking the real one.
- Do NOT meet the module-size budget by compressing formatting: joining statements onto one line, deleting blank lines, or removing doc comments to get under the number is a defect, not a fix. Comments and blank lines are not counted precisely so that explaining the code is always free. If a file has no honest cohesive seam, leave it over budget, raise its documented limit deliberately, and write down why.
- Do NOT write build outputs or package artifacts outside ignored paths.
- Do NOT use mutable GitHub Action references such as tags or branches; pin every third-party action to a full 40-character commit SHA and record the release tag in a comment.
- Do NOT use pre-release Qt Rust bridges, qmetaobject-rs, or mix CXX-Qt family versions; the workspace pins the complete 0.10.0 family.
- Do NOT reduce agreed scope behind “MVP” or “phase 1” language, and do not port third-party source code; external projects are pattern references only.
- Do NOT edit `.factory/gate`, `.factory/protected`, or the paths that file lists — the gate body, `tests/gates/uncovered`, the workflows, and `deny.toml`. Weakening a check is the one change no check can catch, so those diffs are read by a human rather than approved by a run.
