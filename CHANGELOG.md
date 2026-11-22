# Changelog

All notable changes to AllesBuien are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). From 0.2.0 on, each
release's section is the text of its annotated tag and of its GitHub release notes.
Changes not yet released collect under Unreleased, and the release that ships them
renames that section after itself and its date.

## [Unreleased]

### Added

- Text size following. The window zooms its whole content with `omarchy-display-text-size`
  within about two seconds, by the size over Omarchy's default of 12 px, with its minimum size
  growing to match and the zoom capped where that minimum would no longer fit the screen above
  the bar. The window opens at the zoomed size, fitted to the monitor it opens on, and at 12 px
  is exactly as before.

### Changed

- The rain expectation is read from `gps.buienradar.nl/getrr.php`, the address Buienradar's
  free-data terms give for it, where 0.2.0 read the address behind it. Buienradar forwards the
  request, which carries the location, to `gadgets.buienradar.nl`, and the app follows that
  forward over HTTPS only.

### Fixed

- The bar icon no longer keeps the urgent colour after an update until the next click, as 0.2.0's
  did. After a click that opens the floating terminal, the widget checks again every five
  seconds, on every monitor, until the app is current or ten minutes have passed, so the icon
  returns to its ordinary colour within about five seconds of the update finishing.
- The bar widget leaves alone an `allesbuien` that another package provides under its own name,
  such as an `allesbuien-bin`, and opens it on a click, as it already did for any other package's
  binary. 0.2.0's check took that package's files for its own and failed, dimming the icon and
  saying the check had failed. No package provides `allesbuien` that way yet.

## [0.2.0] - 2026-10-29

### Added

- An Omarchy bar launcher. `manifest.json` and `plugin/BarWidget.qml` make this repository an
  Omarchy plugin: a single radar-screen button (the Nerd Font glyph `nf-md-radar`) that focuses the
  open AllesBuien window, launches it, or — while the app is not installed — opens Omarchy's
  floating terminal, downloads the release package `plugin/release.sha256` pins, checks it
  against that digest, and installs it with pacman. A package built against a newer Qt than the
  one installed, which the app may not start on, is refused before the password prompt, with a
  pointer to `omarchy-update`. Once `omarchy plugin update` brings a newer pin, the button turns
  the bar's urgent colour and the same click updates the app; an install the widget does not own
  is only ever launched. It sits beside the built-in weather pill without replacing it.
- Location following. The window adopts `~/.local/state/omarchy/settings/weather.json` within
  about two seconds of `omarchy-weather-location` rewriting or clearing it, and says which of the
  three sources — environment pin, desktop setting, or Amsterdam-West default — answered. A
  location outside the radar's coverage says so on the map, where 0.1.0 only left the marker off.
- Keyboard control of the radar. Space and Enter play and pause wherever the focus is, Left and
  Right step one frame and pause, Home and End jump to the first and the last frame, and Tab
  moves between the play button and the timeline, which show a focus ring. Escape still closes
  the window.
- Packaging metadata: `pkgdesc`, `url`, and an `optdepends` entry naming libnotify as what the
  rain-soon notifications need.

### Changed

- Releases no longer carry an `allesbuien-debug` package. 0.1.0's sat beside the one package to
  install and held only the symbols stripped from the binary, which running the app never needs.
- Radar transport verbs are drawn as icons rather than words, and the rain-rate key gives its
  widest range room to breathe.
- Feed refreshes retry transient failures on a bounded ladder instead of surfacing the first
  error, and a superseded radar refresh is cancelled rather than raced to completion.
- The weather text no longer waits for condition icons. It is shown as soon as the feed answers,
  with the icons already on disk; the rest download concurrently and appear one by one. A slot
  still waiting for its icon shows a placeholder, and the current-conditions one pulses.
- Radar frames download four at a time instead of one after another, and a frame only partly
  written to disk can no longer be mistaken for a cached one.
- The loading state no longer wears an error's clothes: the notice pulses while a refresh is in
  flight, one failure line is shown, and a stale reading says it may be out of date instead of
  calling itself unavailable.
- The current-conditions header leads with the place name and moves the reading's station and
  time up into its corner.

### Fixed

- A rain-soon notification is sent once for each spell of rain. 0.1.0 repeated it at every
  ten-minute refresh while the rain fell, and before that whenever a new forecast moved the start
  or changed the peak. A spell ends when a refresh finds the next two hours dry, or finds the rain
  beyond the threshold once the cooldown has passed since the announcement, so
  `ALLESBUIEN_NOTIFY_COOLDOWN_MIN` now sets the shortest time an announced spell stays open while
  rain is still forecast.
- A current-conditions icon whose download failed comes back on a later refresh instead of
  staying blank until the condition changes.
- A missing temperature reads as a dash instead of an `Unavailable` cut short to `Unavaila…`.
- A rain-soon notification says rain starts about now instead of in zero minutes.
- The window no longer opens on "Weather unavailable", in the error colour, while its first refresh
  is still under way; the pulsing current-conditions icon already says it is loading.
- Every radar legend bucket gets the equal block the key promises, and the station label drops
  the feed's Dutch `Meetstation` and names only the place.
- The radar animation ends where the rain graph below it does, with the graph's cursor on its
  last bar. The radar counts from its latest image, usually five to fifteen minutes old, while
  the rain expectation counts from the present, so two hours of radar often ended a bar or two
  short of the graph. It now fetches two and a half hours, so that a late run still reaches that
  bar, and plays, steps, and jumps to End only as far as it. Without a rain expectation to
  follow, the whole run plays.
- The package carries the licence notices of the code in it that AllesBuien does not own. Beside
  `LICENSE`, the only licence file 0.1.0's package had, `/usr/share/licenses/allesbuien/` now holds
  `THIRD-PARTY-LICENSES`, with the licences and copyright lines of the Rust crates compiled into
  the binary and a note on the Qt header code it uses, and the Rust standard library's
  `COPYRIGHT-library.html`.

### Security

- rustls 0.23.45 fixes RUSTSEC-2026-0285 (GHSA-2mjx-qc3c-rqvc): a TLS 1.3 handshake message sent
  at the wrong encryption level was accepted, instead of ending the connection, when it shared a
  record with the message that changed the keys. 0.1.0 used the affected rustls 0.23.43 for its
  HTTPS requests to Buienradar. The handshake transcript stays authenticated, so a network
  attacker cannot use the flaw to alter or complete a handshake.
- rustls 0.23.46 fixes four more advisories, RUSTSEC-2026-0346 to RUSTSEC-2026-0349. AllesBuien
  is not affected by any of them, and neither was 0.1.0: they need a custom session store that
  formats its entries with `Debug`, Encrypted Client Hello, rustls's optional `brotli` feature, or
  a post-quantum key exchange to downgrade, while the app keeps rustls's own session cache, sends
  no ECH, builds rustls without `brotli`, and uses the ring provider, whose key exchanges (X25519,
  P-256 and P-384) are all classical.
- webpki-roots 1.0.10 and rustls-webpki 0.103.16 fix RUSTSEC-2026-0381 and RUSTSEC-2026-0382
  (both CVE-2026-111943): certificates issued by distrusted emSign intermediates, whose private
  keys are now publicly known, were accepted. 0.1.0 used the affected webpki-roots 1.0.9 and
  rustls-webpki 0.103.15 for its HTTPS requests to Buienradar, so a network attacker holding one
  of those keys can impersonate Buienradar to it. webpki-roots 1.0.10 no longer bundles the
  intermediates' root, emSign ECC Root CA - G3, and rustls-webpki 0.103.16 refuses any chain
  that contains one of their keys.
- tokio 1.53.3 deprecates the `AsyncFd` constructors that RUSTSEC-2026-0386 reports as unsound:
  they accept a file descriptor the caller does not own, which can end in a use-after-free.
  AllesBuien is not affected, and neither was 0.1.0, which used tokio 1.53.1: nothing in the app
  or in the crates it is built from constructs an `AsyncFd`.
- The release build no longer holds a token that can write to this repository. The workflow's
  checkout left the job's token in the workspace's git configuration while makepkg ran every
  dependency's build script and proc macro, any of which could have used it to push to `main`,
  move a tag, or replace a release's assets; 0.1.0 was built that way. The checkout now keeps no
  credentials, and the build runs in a job whose token can only read. A second job, which runs
  none of the build's code, holds the only token that can write: it makes the source archive from
  a checkout of its own, out of the build's reach, and uploads it and the package to the release.

## [0.1.0] - 2026-09-03

### Added

- A native Omarchy weather application for Dutch conditions: current observations, a five-day
  forecast, minute-resolution rain expectation, an animated Buienradar radar with the viewer's
  location marked, rain-soon desktop notifications, and live Omarchy theme integration.

Bron: Buienradar.nl — weather data is used non-commercially with the attribution its terms
require.

[Unreleased]: https://github.com/robi42/AllesBuien/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/robi42/AllesBuien/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/robi42/AllesBuien/releases/tag/v0.1.0
