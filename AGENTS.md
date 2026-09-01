# AGENTS.md — AllesBuien

## Overview

AllesBuien is a native Omarchy weather application built as a Rust workspace with a Qt Quick/CXX-Qt desktop shell. It shows Buienradar current conditions, a five-day forecast, minute rain predictions, an animated Dutch radar, and best-effort rain-soon notifications while following the active Omarchy theme.

## Structure

```
AllesBuien/
├── core/                 Platform-independent weather, radar, notification, config, and theme logic
├── app/                  Qt/CXX-Qt application, controllers, QML UI, assets, and GUI integration tests
├── assets/gen-basemap/   Deterministic pure-Rust Natural Earth basemap generator and checker
├── bin/                  Canonical build, test, and package-install entrypoints
├── pkgbuild/             Arch PKGBUILD, desktop entry, icon, and ignored package artifacts
├── .github/              SHA-pinned CI, release automation, and Dependabot configuration
├── mise.toml             Rust 1.98.0 toolchain declaration
└── lefthook.yml          Pre-commit rustfmt and Clippy checks
```

## Where to Look

| Task | Location | Notes |
|------|----------|-------|
| Configuration and location precedence | `core/src/config.rs` | Environment → optional Omarchy weather settings → Amsterdam-West default |
| Buienradar feed and raintext parsing | `core/src/weather.rs` | `FeedApi` boundary and forecast/current-condition models |
| Radar metadata, frames, and cache pruning | `core/src/radar.rs` | `RadarApi`, generation IDs, bounded run cache |
| Notification rules and adapter | `core/src/notify.rs` | Cooldown/event dedup behind `Notifier` |
| Omarchy theme parsing | `core/src/theme.rs` | Tolerant, path-resolving reads with `Unavailable` fallback state |
| Application construction and dependency injection | `app/src/bootstrap.rs`, `app/src/services.rs` | Qt ownership, Tokio runtime, controller seams |
| Weather bridge | `app/src/weather_controller.rs` | Async feed refresh and QML list model |
| Radar bridge and image delivery | `app/src/radar_controller.rs`, `app/src/image_provider.rs` | Generation-safe playback and `image://radar` provider |
| Theme bridge | `app/src/theme_controller.rs` | Replacement-aware 2-second polling |
| Desktop UI | `app/qml/Main.qml` | Radar, current conditions, forecast, inline statuses, attribution |
| GUI integration tests | `app/tests/` | Exactly one GUI test and one `QGuiApplication` per file/executable |
| Basemap provenance | `assets/BASEMAP-LICENSE` | Pinned Natural Earth source and shipped-image SHA-256 |
| Packaging | `pkgbuild/PKGBUILD`, `pkgbuild/allesbuien.desktop` | Static package metadata and four installed payloads |
| Quality gates | `bin/test`, `lefthook.yml` | Format, Clippy, offscreen tests, purity, coverage, QML lint |

## Commands

Run from the repository root:

```bash
bin/build
bin/test
lefthook run pre-commit
```

## Conventions

- **Core-purity seam:** `allesbuien-core` is platform-independent pure Rust and has zero CXX, CXX-Qt, or Qt dependencies. Qt objects, timers, threads, image delivery, and process adapters belong in `app`.
- **Dependency injection:** Keep network and notification side effects behind the object-safe `FeedApi`, `RadarApi`, and `Notifier` traits. Pass services, paths, location, and timer intervals through the existing construction seams; do not introduce globals.
- **Fakes over mocks:** Tests use small deterministic fake trait implementations with call counters or in-memory values. They do not hit production networks and do not mutate process-global environment state.
- **Qt threading:** HTTP, file reads, and notification dispatch run off the Qt thread; QObject changes return through the CXX-Qt thread queue.
- **Theme-watch re-arm:** Omarchy replaces `~/.local/state/omarchy/current/theme` by removing and renaming a directory. Poll every 2000 ms, re-resolve the path on each changed read, and tolerate disappearance/recreation; an inode-bound watcher cannot survive replacement.
- **Theme fallback:** Rust exposes `themeAvailable`; QML uses `SystemPalette` when unavailable. Theme colors otherwise come from the controller.
- **Radar delivery:** Metadata `times[]` determines frame order and cadence. Transparent frame PNGs are served without Rust-side composition through generation-keyed `image://radar/<generation>/<index>` URLs.
- **Error UX:** Retain last-good data and expose one minimal inline status per view. Malformed or missing optional input must not panic.
- **GUI tests:** Every file directly under `app/tests/` contains exactly one GUI test and constructs exactly one `QGuiApplication`, giving each Qt application its own process.
- **Module size:** No tracked `.rs` file exceeds **250 pure lines** — non-blank, non-comment, measured by `grep -cvE '^[[:space:]]*(//|$)' <file>` and enforced by `bin/test`. Past that a module has outgrown one reviewer's working memory and is doing more than one thing. GUI harnesses under `app/tests/` get **350**, because the one-GUI-test-per-file rule above makes them indivisible; their extra budget is for test doubles, not for a second scenario. The number is a symptom and cohesion is the goal: the only sanctioned fix is extracting a module with a name describing one responsibility, its own module doc, and its own tests.
- **Toolchain and gates:** Use pinned Rust 1.98.0. `bin/test` is the source of truth for workspace verification and keeps core coverage at or above 85%.
- **Packaging:** `pkgver` is static. The canonical package build is exactly `(cd pkgbuild && makepkg -f)` as invoked by `bin/install` and automation.
- **Attribution:** Keep the visible and documented literal `Bron: Buienradar.nl`; Buienradar free data is for non-commercial use with attribution.

## Anti-Patterns

- Do NOT add CXX, CXX-Qt, Qt, app-thread, or UI dependencies to `allesbuien-core`.
- Do NOT put domain or business logic in QML, controller C++, or the image-provider trampoline. The sole C++ carve-out is the existing construction/bridge surface; keep it minimal and logic-free.
- Do NOT block the Qt thread on HTTP, filesystem work, notification dispatch, or Tokio futures.
- Do NOT construct controllers outside the existing library bootstrap/injection seams or introduce global service state.
- Do NOT use mocks when a trait fake can express the scenario; do not call live endpoints from unit tests or mutate process-global environment variables in tests.
- Do NOT place multiple GUI tests or multiple `QGuiApplication` instances in one integration-test file/executable.
- Do NOT decode or compose radar/basemap images in Rust, hardcode radar cadence, retain stale generations, or allow an unbounded radar cache.
- Do NOT use a file/inode watcher for the Omarchy theme directory, fabricate fallback palette values, hard-fail on missing theme keys, or add custom C++ watcher bindings.
- Do NOT hardcode colors in QML; bind controller theme properties or `SystemPalette` fallback values.
- Do NOT add toast/retry frameworks or notification spam; keep one inline error status per view and preserve cooldown/event dedup.
- Do NOT replace Buienradar with wttr.in/Open-Meteo, expand beyond Dutch radar, or remove the required `Bron: Buienradar.nl` attribution.
- Do NOT copy the donor `map-nl-*` image, any unlicensed basemap, or OSM tile-server imagery. Basemaps come only from the pinned Natural Earth public-domain sources through `assets/gen-basemap`.
- Do NOT write to `/home/robi42/Projects/buienradar`, add AllesBuien to the parent `/home/robi42/Projects` repository, or modify `/usr/share/omarchy`.
- Do NOT turn this into a Quickshell widget, touch built-in `omarchy.weather`, add a server/deploy/Docker surface, or publish to the AUR.
- Do NOT create `.SRCINFO`, an AUR remote, a dynamic `pkgver()` function, network-backed PKGBUILD sources, or alternative package-build invocations.
- Do NOT run `makepkg -s`, `makepkg -i`, `makepkg -si`, or `makepkg -fsi`; only `(cd pkgbuild && makepkg -f)` is sanctioned.
- Do NOT invoke bare `sudo` or any command that can wait for a password. Live install mode is user-scope Mode B unless the recorded decision is explicitly changed by its capability probe.
- Do NOT meet the module-size budget by compressing formatting: joining statements onto one line, deleting blank lines, or removing doc comments to get under the number is a defect, not a fix. Comments and blank lines are not counted precisely so that explaining the code is always free. If a file has no honest cohesive seam, leave it over budget, raise its documented limit deliberately, and write down why.
- Do NOT write build outputs or package artifacts outside ignored paths.
- Do NOT use mutable GitHub Action references such as tags or branches; pin every third-party action to a full 40-character commit SHA and record the release tag in a comment.
- Do NOT use pre-release Qt Rust bridges, qmetaobject-rs, or mix CXX-Qt family versions; the workspace pins the complete 0.10.0 family.
- Do NOT reduce agreed scope behind “MVP” or “phase 1” language, and do not port third-party source code; external projects are pattern references only.
