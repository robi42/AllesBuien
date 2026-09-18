# Changelog

All notable changes to AllesBuien are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Each release's section
is the text of its annotated tag and of its GitHub release notes.

## [0.2.0] - 2026-09-18

### Added

- An Omarchy bar launcher. `manifest.json` and `plugin/BarWidget.qml` make this repository an
  Omarchy plugin: a single `󰐷` button that focuses the open AllesBuien window, launches it, or —
  while the app is not installed — opens Omarchy's floating terminal and installs
  `allesbuien-bin` from the AUR. It sits beside the built-in weather pill without replacing it.
- Location following. The window adopts `~/.local/state/omarchy/settings/weather.json` within
  about two seconds of `omarchy-weather-location` rewriting or clearing it, and says which of the
  three sources — environment pin, desktop setting, or Amsterdam-West default — answered.
- Radar transport verbs are drawn as icons rather than words, and the rain-rate key gives its
  widest range room to breathe.
- Packaging metadata: `pkgdesc`, `url`, and an `optdepends` entry naming libnotify as what the
  rain-soon notifications need.

### Changed

- Feed refreshes retry transient failures on a bounded ladder instead of surfacing the first
  error, and a superseded radar refresh is cancelled rather than raced to completion.
- Condition icons are warmed concurrently, and radar runs materialise concurrently and atomically.
- The loading state no longer wears an error's clothes: the notice pulses while a refresh is in
  flight, one failure line is shown, and a stale reading says it may be out of date instead of
  calling itself unavailable.
- The current-conditions header leads with the place name and carries the reading's station and
  time, so a fallback location reads as a default rather than as a chosen place.

### Fixed

- A rain-soon notification says rain starts about now instead of in zero minutes.
- Every radar legend bucket gets the equal block the key promises, and the station label is
  written in the app's language.

## [0.1.0] - 2026-09-03

### Added

- A native Omarchy weather application for Dutch conditions: current observations, a five-day
  forecast, minute-resolution rain expectation, an animated Buienradar radar with the viewer's
  location marked, rain-soon desktop notifications, and live Omarchy theme integration.

Bron: Buienradar.nl — weather data is used non-commercially with the attribution its terms
require.

[0.2.0]: https://github.com/robi42/AllesBuien/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/robi42/AllesBuien/releases/tag/v0.1.0
