# Security policy

AllesBuien is a desktop weather app and an Omarchy bar widget that installs it. The app itself never runs as root and asks for no privileges. Besides the commands README gives you for installing and removing it by hand, the one thing that runs as root is the widget's install and update click, which hands a downloaded package to `sudo pacman -U`. This page says exactly what that click runs, where the package's integrity comes from, and how to report a problem privately.

## Reporting a vulnerability

Report it through this repository's Security tab, [Report a vulnerability](https://github.com/robi42/AllesBuien/security/advisories/new), never in a public issue or pull request. Expect an acknowledgement within a week. A fix ships as a new release, a pin commit that points the widget at it, and a GitHub security advisory, and the release's `CHANGELOG.md` section names it under Security.

## Supported versions

Only the release that `plugin/release.sha256` on `main` pins. Fixes are not backported: a fix is the next release, and the widget installs it once `omarchy plugin update` brings the pin that names it.

## What runs as root, and when

Only two `sudo` commands, both at the end of `plugin/widget.sh`'s install verb: `sudo -v`, which takes the password, and `sudo pacman -U --needed`, which the script spells out once for an install and once for an update. The widget runs the verb itself as you, in Omarchy's floating terminal, when a click finds the app missing, or older than the release its plugin checkout pins, and everything before those two commands runs as you too. In this order:

1. It reads `plugin/release.sha256` from that checkout and refuses any content but one line: a SHA-256 digest, two spaces, and an asset name of the form `allesbuien-<version>-<pkgrel>-x86_64.pkg.tar.zst`.
2. `curl` downloads that asset from this repository's GitHub release over HTTPS only (`--proto =https --proto-redir =https`, TLS 1.2 or newer) into a fresh directory from `mktemp -d`, which is removed when the script ends.
3. `sha256sum --check --strict` checks the download against the pin. A mismatch stops the click before anything else reads the file.
4. `bsdtar` reads which Qt the package was built against from its `.BUILDINFO`, and the click stops if the installed `qt6-declarative` is older, or if either version cannot be read or the two cannot be compared.
5. `sudo -v` asks for the password. The script then decides again whether to install, because another click or another installer can change what is installed while the prompt waits, and stops if the answer or the pin has changed.
6. Only then does `sudo pacman -U --needed` run on the downloaded file, and pacman's own confirmation prompt is the consent.

Nothing is piped into a shell, and GitHub is never asked which release is the latest. A click never downgrades, and an installation the widget does not own, such as a binary another package provides or a user-scope install in `~/.local/bin`, is only ever launched. The packages are not GPG-signed: their integrity rests on the pin alone. README's commands for [installing by hand](README.md#install) check the same digest with `sha256sum -c` before their `sudo pacman -U`.

## Where trust comes from

- **The plugin checkout.** `omarchy plugin add` and `omarchy plugin update` install whatever `main` holds when they run, not the snapshot the Omarchy plugin marketplace verified, so your checkout is `main` as of your last add or update. `git -C ~/.config/omarchy/plugins/io.github.robi42.allesbuien log -1` shows which commit you run. `omarchy plugin update` shows the diff before it applies it, and its changes to `manifest.json`, under `plugin/`, and to any `.gitattributes` are the ones to read: `manifest.json` names the files the shell loads, today only `plugin/BarWidget.qml`, and, in `omarchy.clonedFrom`, a built-in whose trust the shell extends to the plugin; a QML file or `plugin/widget.sh` can load or run others in turn, so any file they reach is one to read too, wherever it sits; `plugin/BarWidget.qml` decides which commands the widget runs; `plugin/widget.sh` and `plugin/release.sha256` decide what a click installs with sudo; and a `.gitattributes` in the directory of any of these files, or in one above it up to the root, can change how git writes them to disk, so a change to one can make them run differently from what the diff shows. A change to one of them that the diff shows only as `Binary files … differ`, or as `(binary file)` where delta draws the diff, is a reason to decline the update: none of them is binary, and a `.gitattributes` from an earlier update can make git hide a change that way.
- **The pin.** The digest in that checkout's `plugin/release.sha256` is the only copy of the package's digest in the tree, and the only check on the bytes a click downloads. `bin/pin-release` writes it once the release exists: it takes the digest GitHub records for the asset, and writes it only after a download of the asset has passed `sha256sum --check` against it.
- **The release.** Releases from v0.2.0 on are immutable: once one is published, nobody can change or replace its assets or move its tag. GitHub can also attest an immutable release; whether it has is checked when each release is made, and nothing on this page relies on that release attestation. v0.1.0 predates the setting and is not immutable, and no pin names it.
- **Build provenance.** Releases after v0.2.0 carry a build-provenance attestation for the package, which `release.yml` signs in a job of its own before it makes the draft. It ties the package's digest to a run of `release.yml`, and through that run to the tag and its commit. With the GitHub CLI, anyone can check a package against it: `gh attestation verify <package> --repo robi42/AllesBuien --signer-workflow robi42/AllesBuien/.github/workflows/release.yml --source-ref refs/tags/v<version> --source-digest <commit>`, where `<commit>` is the one the tag names, which `git ls-remote https://github.com/robi42/AllesBuien 'refs/tags/v<version>^{}'` prints. The commit is there because without it a run for an earlier tag of the same name, whose draft was deleted, passes too. README's [Releasing](README.md#releasing) section runs the same check before a release is published. The attestation says where a package was built, not that the build is benign. It does not say these are the bytes the release published either: a re-run can leave a second attested package for the same tag and commit, and the pin names the published one. The widget's click does not check it. v0.2.0 and v0.1.0 have none.

## Scope

In scope:

- anything that hands pacman other bytes than the pinned asset's, or gets past its prompt;
- a click that downgrades, or that installs over an installation the widget does not own;
- the widget accepting a pin it should refuse;
- injection through the pin line, file names, or the environment;
- races on the temporary directory;
- `.github/workflows/release.yml` and `bin/pin-release`, which build the asset, attest it, and pin it;
- in the app: its TLS connections, hostile Buienradar responses (the feed, the rain expectation, the radar metadata and frames, and the condition icons and their cache), and what it passes to `notify-send`.

Out of scope:

- Buienradar's services, Omarchy, and the Omarchy plugin marketplace; please report those to their owners.
- Advisories in dependencies that Dependabot and the weekly `cargo-deny` audit already track.
