# AllesBuien

AllesBuien is a native weather app for the Omarchy desktop. It combines current Dutch conditions, a five-day forecast, minute-resolution rain expectations, animated radar frames, rain-soon desktop notifications, and live Omarchy theme integration in a floating Qt Quick window.

![AllesBuien showing current conditions, forecast, and Dutch rain radar](assets/screenshot.png)

[Buienradar.nl](https://www.buienradar.nl)

> Buienradar's free weather data is available for non-commercial use with attribution. AllesBuien is intended for personal, non-commercial use.

## Install

Requirements are Rust 1.98.0, Qt 6 Base and Declarative, and Arch's `makepkg`. Run commands from the repository root.

### Mode A: pacman package

Use this only when passwordless sudo is already authorized for the exact pacman install and removal commands. `bin/install` builds the release binary and then runs the one canonical package command, `(cd pkgbuild && makepkg -f)`; it never prompts for privilege itself.

```bash
bin/install
sudo -n /usr/bin/pacman -U --noconfirm "$(pwd)/pkgbuild/allesbuien-0.1.0-1-x86_64.pkg.tar.zst"
```

Rollback the package with the matching non-interactive command:

```bash
sudo -n /usr/bin/pacman -Rns --noconfirm allesbuien
```

Do not use Mode A if either command requires a password. Never run `makepkg` with `-s` or `-i`.

### Mode B: user-scope Cargo install

This is the active mode on the current Omarchy machine. It installs the binary under `~/.local/bin` and the desktop entry, icon, and license under one XDG data root without root privileges:

```bash
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
cargo install --root ~/.local --path app
install -Dm644 pkgbuild/allesbuien.desktop "$DATA_HOME/applications/allesbuien.desktop"
install -Dm644 pkgbuild/allesbuien.svg "$DATA_HOME/icons/hicolor/scalable/apps/allesbuien.svg"
install -Dm644 LICENSE "$DATA_HOME/licenses/allesbuien/LICENSE"
update-desktop-database "$DATA_HOME/applications" || true
```

The Omarchy user wiring in `~/.config/hypr/bindings.lua` binds **SUPER+ALT+W** to launch AllesBuien and applies a floating window rule for app ID `allesbuien`.

## Rollback

Restore the timestamped `bindings.lua` backup created during Omarchy wiring before removing the selected installation mode:

```bash
cp ~/.config/hypr/bindings.lua.bak.<timestamp> ~/.config/hypr/bindings.lua
hyprctl reload
```

For Mode B, uninstall the Cargo binary and remove all user payloads from the same resolved XDG data directory:

```bash
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
cargo uninstall --root ~/.local allesbuien
rm -f "$HOME/.local/bin/allesbuien" \
  "$DATA_HOME/applications/allesbuien.desktop" \
  "$DATA_HOME/icons/hicolor/scalable/apps/allesbuien.svg"
rm -rf "$DATA_HOME/licenses/allesbuien"
update-desktop-database "$DATA_HOME/applications" || true
```

For Mode A, restore `bindings.lua` as above and run the pacman removal command from the Mode A section.

## Development

Install the pinned Rust toolchain and run the repository's complete quality gate:

```bash
mise install
bin/test
```

Build the release application with:

```bash
bin/build
```

`bin/test` checks the per-file module-size budget, rustfmt, Clippy with warnings denied, offscreen Qt workspace tests, the zero-CXX core dependency seam, at least 85% line coverage for `allesbuien-core`, the pinned `cargo-llvm-cov` version, and Qt 6 QML lint.

## Data and basemap licensing

- Weather observations, forecasts, and radar: **Buienradar.nl**. Free-data use requires attribution and is limited to non-commercial use. The required attribution is visible in the app and this README.
- Basemap: **Natural Earth 1:10m vector data**, public domain. Natural Earth permits use, modification, redistribution, and commercial use without required credit.
- The exact Natural Earth commit, source URLs, terms link, and generated basemap SHA-256 are recorded in [`assets/BASEMAP-LICENSE`](assets/BASEMAP-LICENSE).
- Application code: [MIT License](LICENSE).
