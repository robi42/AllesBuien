# AllesBuien

Dutch weather and rain radar for Omarchy.

AllesBuien is a native weather app for the Omarchy desktop. It combines current Dutch conditions, a five-day forecast, the rain expected over the next two hours in five-minute steps, animated radar frames, rain-soon desktop notifications, and live Omarchy theme integration in one Qt Quick window.

![AllesBuien showing current conditions, forecast, and Dutch rain radar](preview.png)

Weather data and radar imagery: [Buienradar.nl](https://www.buienradar.nl)

> Buienradar's free weather data is available for non-commercial use with attribution. AllesBuien is intended for personal, non-commercial use.

## Install

AllesBuien is two pieces: the native application, and an Omarchy bar widget that opens it. You need Omarchy on x86_64, recent enough to have `omarchy plugin`; it also ships the `curl`, `sha256sum`, and `pacman` the widget uses. The package pulls in Qt 6 (`qt6-base` and `qt6-declarative`), and `libnotify` is optional: only rain-soon notifications need it.

Add the plugin, then let it install the app:

```bash
omarchy plugin add https://github.com/robi42/AllesBuien.git --enable
```

The widget appears in the bar's center section as a radar-dish icon, the Nerd Font glyph `nf-md-radar`. While the application is missing the icon is dimmed, and one click opens Omarchy's floating terminal, downloads the release package that the plugin's `plugin/release.sha256` pins from this repository's [releases page](https://github.com/robi42/AllesBuien/releases), checks it against the SHA-256 digest in that file, and installs it with `sudo pacman -U` — the password prompt and pacman's own confirmation appear in that terminal — then starts the app. A download that fails or does not match the digest stops before pacman runs, and the terminal says so. Every later click focuses the open window, or launches it if none is open. A copy of the plugin that pins no release yet says so on the click and points here; build from source, below, instead.

Or install the application by hand first and add the widget afterwards. Download [`plugin/release.sha256`](plugin/release.sha256) from this repository, and the package it names from the releases page, into an empty directory, check the package against the pin, and install that package:

```bash
cd "$(mktemp -d)"
curl -fsSLO https://raw.githubusercontent.com/robi42/AllesBuien/main/plugin/release.sha256
package=$(awk '{ print $2 }' release.sha256)
curl -fsSLO "https://github.com/robi42/AllesBuien/releases/download/v$(cut -d- -f2 <<<"$package")/$package"
sha256sum -c release.sha256 && sudo pacman -U "./$package"
omarchy plugin add https://github.com/robi42/AllesBuien.git --enable
```

`sha256sum -c` has to print `OK`, and the `&&` runs pacman only if it does. Anything else means the file is not the release this repository pinned, so do not install it.

`--enable` asks which bar section to place the widget in, with the center preselected. To skip the question, or to move the widget later:

```bash
omarchy plugin enable io.github.robi42.allesbuien --after omarchy.weather
omarchy bar move io.github.robi42.allesbuien --section right
```

The widget only opens, focuses, installs, or updates the application. The window itself, its notifications, and `omarchy-weather-location` behave exactly as they do without the widget, and the application runs perfectly well with no plugin installed at all.

## Updating

The widget installs and updates only the release its own copy pins, and a newer pin reaches it only with the plugin itself:

```bash
omarchy plugin update io.github.robi42.allesbuien
```

That shows the change as a diff and asks before applying it. Once the updated plugin pins a newer release than the installed `allesbuien` package, the icon turns the bar's urgent colour, with an "update ready" tooltip. Click it, and the same floating terminal shows the installed and the new version, downloads and checks the package, and lets pacman ask before replacing it. Updated or not, the click ends with AllesBuien open or focused. An AllesBuien that was already running keeps the old version until you restart it; the terminal says so, and nothing is closed for you.

The widget never asks GitHub for the latest release. It leaves alone any installation it does not own — an `allesbuien` binary another package provides, or a user-scope install in `~/.local/bin` ([Mode B](#mode-b-user-scope-cargo-install)) — and any `allesbuien` package already at or past the pin: those it only opens or focuses.

## Alongside the Omarchy weather widget

The launcher does not replace Omarchy's own weather pill, and it is not a clone of it: both can sit in the bar at the same time, neither disables or reorders the other, and the pill keeps its panel, its settings, and its IP auto-detection. The two read the same desktop location setting, and AllesBuien only ever reads it — `omarchy-weather-location` remains the one place that setting is changed.

The glyphs are deliberately unlike each other. The pill draws the current condition, a cloud or a sun or a raindrop; the launcher draws a radar dish, the same one in every kind of weather, so the bar never shows the same pictogram twice.

## Location

AllesBuien reads the location Omarchy already knows, so there is nothing extra to configure and no second place to change it:

```bash
omarchy-weather-location --set Groningen 53.22,6.57
omarchy-weather-location --clear
```

The open window follows either command within about two seconds. The place name, the marker on the radar, and the weather it fetches all move together; no restart is involved, and the desktop's own weather widget follows the same file at the same time.

Three sources answer in order. An `ALLESBUIEN_LAT`/`ALLESBUIEN_LON` pair, with `ALLESBUIEN_LOCATION_LABEL` to name it, pins this app alone and outranks everything after it, including later changes to the Omarchy setting. Omarchy's `~/.local/state/omarchy/settings/weather.json` decides otherwise, provided it carries a name and both coordinates. Failing both, the window falls back to Amsterdam-West. The header says which of the three answered, so a fallback reads as a default rather than as a place someone chose.

Two behaviours are deliberate, and both follow from this being a Dutch radar viewer rather than a general weather app:

Unlike Omarchy's own weather widget, **AllesBuien does not auto-detect your location from your IP address.** That widget can render any city on Earth, so detection is a good default for it. This one draws Buienradar's Dutch radar and nothing else, so an IP resolving to Berlin would produce a confident-looking window over a permanently empty map. A sensible Dutch default, labelled as a default, is the better wrong answer.

And a location outside the radar's coverage says so on the map, rather than leaving a blank raster that looks like a failed download. `omarchy-weather-location --set Malibu` — a name with no coordinates — is treated as no setting at all, because geocoding it would mean owning a copy of a setting the desktop already owns.

## Notifications

When rain is expected to start within 30 minutes, AllesBuien announces it once through `notify-send`, which `libnotify` provides, and does not repeat that announcement for an hour; rain whose start or strength changes is announced again. Three environment variables, read when the app starts, tune this:

- `ALLESBUIEN_NOTIFY`: notifications are on unless this is set to something other than `1`, `true`, `yes`, or `on`.
- `ALLESBUIEN_NOTIFY_THRESHOLD_MIN`: how many minutes ahead rain counts as news, 30 by default.
- `ALLESBUIEN_NOTIFY_COOLDOWN_MIN`: how many minutes the same announcement stays quiet, 60 by default.

## Keys

- **Space** or **Enter** plays and pauses the radar, wherever the focus is.
- **Left** and **Right** step one frame back or forward and pause.
- **Home** and **End** jump to the first and the latest frame.
- **Tab** moves between the play button and the timeline, which show a ring while they have keyboard focus.
- **Escape** closes the window.

## Optional Hyprland wiring

Nothing above touches your Hyprland configuration. If you want a keybinding and a floating window, add these two lines to `~/.config/hypr/bindings.lua` yourself, then reload:

```lua
o.bind("SUPER + ALT + W", "AllesBuien", { launch = "allesbuien", focus = "^allesbuien$" })
o.window("allesbuien", { float = true })
```

```bash
hyprctl reload
```

Without the second line the window tiles like any other, which is fine; the app does not depend on either line.

## Build from source

Building needs Rust 1.98.0, which `mise install` provides, and `qt6-base` and `qt6-declarative`, which on Arch include the development headers. Mode A also needs Arch's `makepkg`, [`just`](https://github.com/casey/just), and a pacman package that provides `rust`, such as `rustup` (`sudo pacman -S just rustup`). makepkg checks the PKGBUILD's build dependencies against pacman's database, which knows nothing of mise's toolchain; the build itself still uses Cargo 1.98.0, the only version `bin/build` accepts. Clone the repository and run every command from its root:

```bash
git clone https://github.com/robi42/AllesBuien.git && cd AllesBuien
mise install
```

### Mode A: pacman package

At your own terminal, where sudo can ask for your password:

```bash
just package
just install
```

`just install` installs the package `just package` built, leaving out the debug package makepkg's default options add. It refuses while a Mode B install is in `~/.local/bin`, which comes first on `PATH` and would hide the package; remove that one first, as [Uninstall](#uninstall) says.

Unattended, use this only when passwordless sudo is already authorised for the exact pacman install and removal commands. `bin/install` builds the release binary and runs the one canonical package command, `(cd pkgbuild && makepkg -f)`. It installs nothing and never prompts for privilege, and the command it prints at the end is [Mode B](#mode-b-user-scope-cargo-install)'s, not a step of this route.

```bash
bin/install
sudo -n /usr/bin/pacman -U --noconfirm "$(pwd)/pkgbuild/allesbuien-0.2.0-1-x86_64.pkg.tar.zst"
```

Remove the package again with the matching non-interactive command:

```bash
sudo -n /usr/bin/pacman -Rns --noconfirm allesbuien
```

Do not use the unattended form if either command would ask for a password. Either way, build with `makepkg -f` alone, as `just package` and `bin/install` do: installing is the separate step above, not `makepkg -i`.

### Mode B: user-scope Cargo install

This mode needs no root privileges. It installs the binary under `~/.local/bin` and the desktop entry, icon, and licence under one XDG data root:

```bash
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
cargo install --locked --root ~/.local --path app
install -Dm644 pkgbuild/allesbuien.desktop "$DATA_HOME/applications/allesbuien.desktop"
install -Dm644 pkgbuild/allesbuien.svg "$DATA_HOME/icons/hicolor/scalable/apps/allesbuien.svg"
install -Dm644 LICENSE "$DATA_HOME/licenses/allesbuien/LICENSE"
```

`--locked` builds the dependency versions `Cargo.lock` records, the ones CI tested; without it, `cargo install` resolves them afresh.

## Uninstall

Remove the widget first, then the application, in this order:

```bash
omarchy plugin remove io.github.robi42.allesbuien
```

For the package the widget's click installs, or one built from this repository with Mode A:

```bash
sudo pacman -Rns allesbuien
```

For Mode B, uninstall the Cargo binary and remove all user payloads from the same resolved XDG data directory:

```bash
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
cargo uninstall --root ~/.local allesbuien
rm -f "$HOME/.local/bin/allesbuien" \
  "$DATA_HOME/applications/allesbuien.desktop" \
  "$DATA_HOME/icons/hicolor/scalable/apps/allesbuien.svg"
rm -rf "$DATA_HOME/licenses/allesbuien"
```

Finally, only if you added the optional Hyprland wiring above, delete those two lines from `~/.config/hypr/bindings.lua` and run `hyprctl reload`.

## Dependencies

- For the widget: the application itself, either the release package its click installs or a build from this repository.
- `qt6-base` and `qt6-declarative`, which the package pulls in.
- `libnotify`, optional; without it the app runs and simply never notifies.
- Omarchy itself, which ships `curl`, `sha256sum`, and `pacman` for the widget's install and update click, and `hyprctl` plus `jq` for `omarchy-launch-or-focus`. No AUR helper is needed.
- Network access to Buienradar for observations, forecasts, and radar frames.

## Development

Install the pinned tools and the pre-commit hook, then run the repository's complete quality gate:

```bash
mise install
bin/test
```

`bin/test` also needs `cargo-llvm-cov` at the version `.llvm-cov-version` names, and the toolchain's `llvm-tools-preview` component, neither of which `mise.toml` provides:

```bash
cargo install --locked cargo-llvm-cov --version "$(cat .llvm-cov-version)"
rustup component add llvm-tools-preview
```

`mise install` provides Rust 1.98.0, actionlint, shellcheck, and lefthook 2.1.16, then, in the main checkout, runs `lefthook install`, which writes the pre-commit hook into `.git/hooks`, where every worktree shares it. In a linked worktree `mise install` leaves the hook alone, but `lefthook install` run by hand in one still rewrites the hook every worktree shares from that worktree's `lefthook.yml`. The hook runs `cargo fmt --all --check` and `bin/check-fast` before each commit. A commit fails when the hook finds no lefthook, or first finds one older than 2.1.16. The hook never reinstalls itself, so after a change to `lefthook.yml` beyond its jobs, such as a new hook, run `mise install` in the main checkout again. Because `mise.toml` sets up that hook, mise reads it only from a trusted checkout: `mise install` trusts the one it runs in, and a linked worktree shares its main checkout's trust.

Build the release application with:

```bash
bin/build
```

[`just`](https://github.com/casey/just) lists shortcuts over these for people: `just ci` runs `bin/test`, `just check` runs `bin/check-fast`, `just run` starts the app from the checkout, and `just package` and `just install` are Mode A above. Nothing in the gate runs them, so `just` is not among the tools `mise.toml` pins.

Among other gates, `bin/test` checks the per-file module-size budget, rustfmt, Clippy with warnings denied, ShellCheck and actionlint, the offscreen Qt workspace tests and their re-run in four time zones, the zero-CXX core dependency seam, at least 90% line coverage for `allesbuien-core`, the pinned `cargo-llvm-cov` version, Qt 6 QML lint and the QML type contract, the bar widget's install and update script against fake `pacman`, `curl`, and `sha256sum`, and finally that every gate still fails on its known-bad inputs.

Bugs and ideas are welcome as [GitHub issues](https://github.com/robi42/AllesBuien/issues). The maintainer tracks work in-repo as [beads](https://github.com/steveyegge/beads): `bd ready` lists what is actionable, `bd show <id>` reads one.

`.factory/gate` is the software factory's definition of done and wraps `bin/test`, so an agent-driven branch is held to the same gate CI runs. `.factory/protected` lists the paths whose diffs a human has to read before that branch can land: the gate's scripts, the fixture allowlist, the workflows and `deny.toml`, the justfile, whose `just install` runs sudo, and mise's and lefthook's config files, which steer the pre-commit hook every worktree shares, with the root `.gitignore`, whose re-includes keep a global ignore from hiding a local one. A branch that drops `no_auto_install` or the postinstall's guard, or adds a `lefthook-local` file that turns `no_auto_install` off, has rewritten that hook at its first commit or `mise install` in its worktree, before Factory flags it; `mise install` in the main checkout writes it back.

### Plugin checks

`bin/test` proves the widget's shell half, `plugin/widget.sh`, and the committed pin, against fake `pacman`, `curl`, and `sha256sum` in its workspace tests, and holds only a text belt over the rest — the manifest's shape, entry points that are tracked files, no symlink anywhere, no literal colour — because CI runs in a bare Arch container, and `qs.Ui`, `omarchy-plugin-validate`, and the shell itself exist only on an Omarchy desktop. Run these three checks by hand before every tag, and again on the pin commit that follows it.

Which states a checked commit can show depends on its pin. With no pin at all, as before the first release's pin commit, an absent application gets the message pointing to *Install* and an installed one is launched or focused. Once a release is pinned, the click also installs it, and an older package shows update ready; between a later version bump and its pin commit, the pin still names the previous release.

Validate what `omarchy plugin add` would clone: a clean checkout of `main` from GitHub, because the validator walks ignored `target/` too and refuses every symlink it finds, and because the commit these checks pass on is the one a release tags. The `rev-parse` line prints it; note it down for the tag:

```bash
rm -rf /tmp/allesbuien-candidate
git clone -q https://github.com/robi42/AllesBuien.git /tmp/allesbuien-candidate
git -C /tmp/allesbuien-candidate rev-parse HEAD
omarchy plugin validate /tmp/allesbuien-candidate
```

Keep the clone until the end: the click-through below installs it, and it is deleted only after the widget has been removed again.

Lint the widget. `qs.Ui` resolves through a module root named `qs`, so the shell directory has to be reachable under that name:

```bash
mkdir -p /tmp/qmlroot && ln -sfn /usr/share/omarchy/shell /tmp/qmlroot/qs
/usr/lib/qt6/bin/qmllint -I /tmp/qmlroot /tmp/allesbuien-candidate/plugin/BarWidget.qml
```

Two warnings are expected, and Omarchy's own first-party plugins produce both: `bar.run` is not visible on the `QtObject` the bar is declared as, and `QProcess::ExitStatus` has no QML type for `onExited`'s second parameter.

Then install the candidate and try it in a real bar:

```bash
omarchy plugin add /tmp/allesbuien-candidate --enable
```

Before clicking anything, make the install step inert. A click while the application is absent, or while an update is ready, runs `sudo -v` and `sudo pacman -U`, which ask for a password and install a package; proving the press path does not need that, and the real install click belongs to whoever is publishing the release. Edit the copy the shell actually loads — `omarchy plugin add` clones the candidate, so editing the candidate changes nothing now, and an uncommitted edit there would never have been cloned in the first place:

```bash
widget=~/.config/omarchy/plugins/io.github.robi42.allesbuien/plugin/widget.sh
sed -i 's/sudo -v/echo dry-run sudo -v/; s/sudo pacman -U/echo dry-run pacman -U/' "$widget"
grep -c 'echo dry-run' "$widget"   # 3: the password prompt, the install, and the update
```

The widget runs the script afresh on every probe and click, so the edit takes effect at once. An edited installed copy also stops `omarchy plugin update` from fast-forwarding it, which is one more reason these checks end by removing the plugin.

Now click the icon and watch the states: dimmed while the application is absent, with the install tooltip while a release is pinned and one pointing to *Install* while none is, and a click opening the floating terminal, which with a pin downloads, checks, and echoes the dry run instead of installing, and without one says nothing is pinned yet; while the repository is private the download fails, and the terminal says so and installs nothing. Then: lit within five seconds of an install finishing elsewhere (`cargo install --locked --root ~/.local --path app` is enough); launching the window when none is open; focusing the open window instead of opening a second one. None of these may need a shell restart or a plugin reload. Check the last two with another window open whose title mentions AllesBuien — this repository's GitHub page, as the browser's active tab, will do, but not a window titled exactly AllesBuien, which is the application's own title: the click has to launch or focus the application itself, never that window.

On a pin commit, check update ready too. It needs an `allesbuien` package older than the pin to own the binary PATH finds, so in your own terminal install v0.1.0's package and move the Mode B binary aside, since `~/.local/bin` comes before `/usr/bin` on the login PATH and an unowned binary must keep launching rather than being updated:

```bash
cd "$(mktemp -d)"
gh release download v0.1.0 --repo robi42/AllesBuien --pattern 'allesbuien-0.1.0-*'
sudo pacman -U ./allesbuien-0.1.0-*.pkg.tar.zst
mv ~/.local/bin/allesbuien ~/.local/bin/allesbuien.mode-b
omarchy-shell shell rescanPlugins
```

The rescan recreates the widget, and its first probe turns the icon the bar's urgent colour with the update tooltip. A click shows `0.1.0-1` and the pinned version, downloads, checks, echoes the dry run instead of running pacman, and opens or focuses AllesBuien. While the repository is private that download fails: the terminal says so, installs nothing, and still opens the installed version. That is the failure check, and naming `9.9.9-1` in the installed copy's `release.sha256` reproduces it at any time; the successful half belongs to the dry run once the repository is public.

Finish by putting the desktop back as it was, the last two lines only if you ran the update-ready check:

```bash
omarchy plugin remove io.github.robi42.allesbuien
rm -rf /tmp/allesbuien-candidate
sudo pacman -Rns allesbuien
mv ~/.local/bin/allesbuien.mode-b ~/.local/bin/allesbuien
```

### Releasing

Nine files carry the version and `release.yml` compares only three of them against the tag, so bump all nine; `bin/check-fast`'s `release version` gate refuses a commit that leaves one behind: `app/Cargo.toml`, `core/Cargo.toml`, `app/harness/Cargo.toml`, `app/testkit/Cargo.toml`, `assets/gen-basemap/Cargo.toml`, the five versioned workspace entries in `Cargo.lock` (`cargo update --workspace --offline`), `pkgver=` in `pkgbuild/PKGBUILD`, `"version"` in `manifest.json`, and the package filename in this README.

Then write the release's `CHANGELOG.md` section, dated the day you tag, and land it on `main`. Once CI is green there, run the plugin checks above, which print the commit they checked. Before tagging, enable GitHub's immutable releases for the repository, so a published release's tag and assets can never change under a pin; the release workflow's action creates the release as a draft, uploads the assets, and only then publishes it, which is the order immutable releases require. Tag the checked commit, not whatever happens to be checked out locally, with the section as the message under a first line naming the release, the way `v0.1.0`'s message begins:

```bash
git fetch origin
sha=$(git rev-parse origin/main)   # must be the commit the plugin checks printed
version=$(git show "$sha:app/Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)
{ printf 'AllesBuien %s\n\n' "$version"
  git show "$sha:CHANGELOG.md" | awk -v h="## [$version]" 'index($0, h) == 1 { f = 1; next } /^## \[/ { f = 0 } f'
} > "/tmp/allesbuien-v$version.md"
git tag -a "v$version" --cleanup=whitespace -F "/tmp/allesbuien-v$version.md" "$sha" &&
  git cat-file -p "v$version"
```

`--cleanup=whitespace` is what keeps the section's `###` headings: git's default cleanup takes every line starting with `#` for a comment and deletes it. Before pushing, check that `git cat-file -p` printed the section with its headings and, on its `object` line, the commit the plugin checks printed, because the pushed tag is what builds and attaches the release assets. If either is wrong, or `git tag` says the tag already exists because an earlier attempt left it behind, do not push: delete it with `git tag -d "v$version"`, put right what was wrong, and run the block above again. Then, in the same terminal, push it:

```bash
git push origin "refs/tags/v$version"
```

The workflow leaves the release's description empty, and GitHub then shows the tag message as preformatted text, so once its run is green give the release the same notes as Markdown, read back from the tag itself; in a new terminal, set `version` first:

```bash
git tag -l --format='%(contents:body)' "v$version" | gh release edit "v$version" --notes-file -
```

Once the asset exists, pin it. From the repository root, on an up-to-date `main`:

```bash
bin/pin-release
```

It reads the digest GitHub records for this version's package, downloads the package through `gh`, which is authenticated and so works while the repository is private, checks the download with `sha256sum -c`, and only then writes `plugin/release.sha256` and prints the commit to make. Read the line it wrote, commit it as printed, and push. That commit is what `omarchy plugin update` delivers and what the marketplace verifies. Do not re-run the release workflow after that: it would upload a rebuild with different bytes under the same name, and the pin would stop matching; with immutable releases enabled, GitHub refuses that upload outright.

Then run the plugin checks again on the pin commit, the click-through's install and update-ready states included. If the repository is still private, make it public now, since the widget's own download is anonymous, and repeat the click-through's dry run so the real download succeeds and verifies. Finally submit the pin commit's SHA to the marketplace, or re-verify the listing with it; the listing tracks the commit it was scanned at, not `main`.

Publishing `allesbuien-bin` to the AUR comes later, as an additional channel, once AUR account registration reopens; the widget does not depend on it.

## Data and basemap licensing

- Weather observations, forecasts, and radar: **Buienradar.nl**. Free-data use requires attribution and is limited to non-commercial use. The required attribution is visible in the app and this README.
- Basemap: **Natural Earth 1:10m vector data**, public domain. Natural Earth permits use, modification, redistribution, and commercial use without required credit.
- The exact Natural Earth commit, source URLs, terms link, and generated basemap SHA-256 are recorded in [`assets/BASEMAP-LICENSE`](assets/BASEMAP-LICENSE).
- Application code: [MIT License](LICENSE).
