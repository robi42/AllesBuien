# Changelog

All notable changes to AllesBuien are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). From 0.2.0 on, each
release's section is the text of its annotated tag and of its GitHub release notes.

## [0.2.0] - 2026-10-03

### Added

- An Omarchy bar launcher. `manifest.json` and `plugin/BarWidget.qml` make this repository an
  Omarchy plugin: a single radar-dish button (the Nerd Font glyph `md-radar`) that focuses the
  open AllesBuien window, launches it, or — while the app is not installed — opens Omarchy's
  floating terminal, downloads the release package `plugin/release.sha256` pins, checks it
  against that digest, and installs it with pacman. Once `omarchy plugin update` brings a newer
  pin, the button turns the bar's urgent colour and the same click updates the app; an install
  the widget does not own is only ever launched. It sits beside the built-in weather pill
  without replacing it.
- Location following. The window adopts `~/.local/state/omarchy/settings/weather.json` within
  about two seconds of `omarchy-weather-location` rewriting or clearing it, and says which of the
  three sources — environment pin, desktop setting, or Amsterdam-West default — answered. A
  location outside the radar's coverage says so on the map, where 0.1.0 only left the marker off.
- Keyboard control of the radar. Space and Enter play and pause wherever the focus is, Left and
  Right step one frame and pause, Home and End jump to the first and the latest frame, and Tab
  moves between the play button and the timeline, which show a focus ring. Escape still closes
  the window.
- Packaging metadata: `pkgdesc`, `url`, and an `optdepends` entry naming libnotify as what the
  rain-soon notifications need.

### Changed

- Radar transport verbs are drawn as icons rather than words, and the rain-rate key gives its
  widest range room to breathe.
- Feed refreshes retry transient failures on a bounded ladder instead of surfacing the first
  error, and a superseded radar refresh is cancelled rather than raced to completion.
- The weather text no longer waits for condition icons. It is shown as soon as the feed answers,
  with the icons already on disk; the rest download concurrently and appear one by one. A slot
  still waiting for its icon shows a placeholder, and the current-conditions one pulses.
- Radar frames download four at a time instead of one after another, and a frame whose download
  was interrupted can no longer be mistaken for a cached one.
- The loading state no longer wears an error's clothes: the notice pulses while a refresh is in
  flight, one failure line is shown, and a stale reading says it may be out of date instead of
  calling itself unavailable.
- The current-conditions header leads with the place name and moves the reading's station and
  time up into its corner.

### Fixed

- A current-conditions icon whose download failed comes back on a later refresh instead of
  staying blank until the condition changes.
- A missing temperature reads as a dash instead of an `Unavailable` cut short to `Unavaila…`.
- A rain-soon notification says rain starts about now instead of in zero minutes.
- Every radar legend bucket gets the equal block the key promises, and the station label is
  written in the app's language.

### Security

- rustls 0.23.45 fixes RUSTSEC-2026-0285 (GHSA-2mjx-qc3c-rqvc): a TLS 1.3 handshake message sent
  at the wrong encryption level was accepted, instead of ending the connection, when it shared a
  record with the message that changed the keys. 0.1.0 used the affected rustls 0.23.43 for every
  HTTPS request to Buienradar. The handshake transcript stays authenticated, so a network attacker
  cannot use the flaw to alter or complete a handshake.

## [0.1.0] - 2026-09-03

### Added

- A native Omarchy weather application for Dutch conditions: current observations, a five-day
  forecast, minute-resolution rain expectation, an animated Buienradar radar with the viewer's
  location marked, rain-soon desktop notifications, and live Omarchy theme integration.

Bron: Buienradar.nl — weather data is used non-commercially with the attribution its terms
require.

[0.2.0]: https://github.com/robi42/AllesBuien/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/robi42/AllesBuien/releases/tag/v0.1.0
