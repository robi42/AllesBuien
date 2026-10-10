#!/usr/bin/env bash
set -euo pipefail

# The shell half of the AllesBuien bar widget. BarWidget.qml draws a button and
# maps this script's answers onto it; every decision about the app, and every
# step of installing or updating it, lives here, where it can be read, linted,
# and tested against fake pacman, curl, and sha256sum without a desktop
# (tests/widget/run.sh). The widget launches, focuses, installs or updates the
# release that release.sha256 beside this file pins, and nothing else.
#
#   widget.sh probe     decide, and answer with the exit code alone:
#                         0   launch or focus: installed and current, installed
#                             but not ours to update, or installed with no pin
#                         10  absent, and a release is pinned: a click installs
#                         11  absent, and nothing usable is pinned: a click says
#                             where to read about installing by hand
#                         12  ours and older than the pin: a click updates
#                       Any other code is this script failing; the widget
#                       treats it as 11, so the next click opens a terminal
#                       where the failure can be read.
#   widget.sh launch    focus the open window, or start the app
#   widget.sh install   what Omarchy's floating terminal runs for 10, 11 and 12:
#                       decide again, then download, verify, and hand the
#                       package to pacman, whose own prompt is the consent
#
# The pin is one line in `sha256sum --check` format: a SHA-256 digest, two
# spaces, and the release asset's name, allesbuien-<version>-<pkgrel>-x86_64
# .pkg.tar.zst. The version, the tag, and the download URL all come from that
# name, and the digest exists nowhere else. The pin is written after a release's
# asset exists, so between a version bump and that commit it names the previous
# release. The widget never asks GitHub which release is the latest: what a
# click installs is what the reviewed plugin snapshot pins, and
# `omarchy plugin update` is the only way a newer pin arrives.

readonly repository=https://github.com/robi42/AllesBuien
readonly readme_install=$repository#install

# Absolute, so the pin can still be named after the verify step changes into
# the temp dir, and so the detached launch finds this script from anywhere.
self=$(readlink -f -- "${BASH_SOURCE[0]}")
readonly self
readonly pin_file=${self%/*}/release.sha256

# Filled by read_pin and decide.
pin_asset=
pin_version=
pin_problem=
installed_version=
decision=
tmp=

# Sets pin_asset and pin_version from a usable pin and succeeds; fails when
# there is none. A pin that exists but cannot be read, or does not have the one
# line's shape, is no pin at all - installing what a malformed line seems to
# name would be guessing - and pin_problem says which, for the person reading
# the terminal and on stderr for whoever reads the probe's.
read_pin() {
  local contents pattern
  pin_asset=
  pin_version=
  pin_problem=
  if [[ ! -e $pin_file ]]; then
    return 1
  fi
  if [[ ! -f $pin_file || ! -r $pin_file ]] || ! contents=$(<"$pin_file"); then
    pin_problem="$pin_file cannot be read"
  else
    pattern='^[0-9a-f]{64}  (allesbuien-([^-]+-[0-9]+)-x86_64\.pkg\.tar\.zst)$'
    if [[ $contents != *$'\n'* && $contents =~ $pattern ]]; then
      pin_asset=${BASH_REMATCH[1]}
      pin_version=${BASH_REMATCH[2]}
      return 0
    fi
    pin_problem="$pin_file is not one sha256sum line naming an allesbuien package: ${contents%%$'\n'*}"
  fi
  printf 'widget.sh: %s\n' "$pin_problem" >&2
  return 1
}

# Whether the allesbuien pacman package owns the binary PATH resolves to. Only
# that package is ours to update: a binary another package owns, such as an
# allesbuien-bin from the AUR, or one pacman does not own at all, such as a
# Mode B install in ~/.local/bin that shadows a package further down PATH, is
# left alone. Asked as membership in the one package's file list rather than
# with pacman -Qo, which reads every installed package's list and took over a
# second on an unowned path - and the probe runs on every click. The list is
# captured rather than piped into grep, because grep -q exits at its first
# match and pipefail would turn that match into a SIGPIPE failure. No pacman
# on PATH fails the capture too, and so counts as not ours.
owned_by_us() {
  local files resolved
  if ! files=$(pacman -Qlq allesbuien 2>/dev/null); then
    return 1
  fi
  resolved=$(readlink -f "$1")
  grep -qxF -- "$resolved" <<<"$files"
}

# Sets decision to one of the probe's exit codes, and installed_version when
# the binary is ours. Only an installed version older than the pin by vercmp,
# pkgrel included, is an update; equal or newer is left as it is, so a click
# never downgrades.
decide() {
  local binary line order
  installed_version=
  if ! binary=$(command -v allesbuien); then
    if read_pin; then
      decision=10
    else
      decision=11
    fi
    return 0
  fi
  if ! read_pin || ! owned_by_us "$binary"; then
    decision=0
    return 0
  fi
  line=$(pacman -Q allesbuien)
  installed_version=${line##* }
  order=$(vercmp "$installed_version" "$pin_version")
  if [[ $order == -1 ]]; then
    decision=12
  else
    decision=0
  fi
}

launch_verb() {
  # Focus the open window if there is one, else launch, in the command shape
  # Omarchy's launch_sole helper builds (its Obsidian binding: "^obsidian$").
  # The helper tests the pattern against every window's class and title, any
  # case, so a bare word would also focus a terminal or browser whose title
  # mentions AllesBuien; anchored, only a class or whole title of AllesBuien
  # answers. uwsm-app gives the app a scope of its own.
  exec omarchy-launch-or-focus '^allesbuien$' 'uwsm-app -- allesbuien'
}

# Starts the launch verb and returns at once, as omarchy-install-and-launch
# does, so the terminal is not held open for the app's lifetime. The subshell
# is what the verb's exec replaces, and a subshell does not run this script's
# EXIT trap, so the temp dir is still removed when this script ends.
launch_detached() {
  (bash "$self" launch >/dev/null 2>&1 &)
}

no_pin_message() {
  if [[ -n $pin_problem ]]; then
    printf 'This copy of the AllesBuien plugin carries a release pin it cannot use:\n  %s\n' "$pin_problem"
    printf 'That is a defect in the plugin, not on this machine, so nothing was installed.\n'
  else
    printf 'This copy of the AllesBuien plugin pins no release yet, so there is nothing to install from here.\n'
  fi
  printf 'Install AllesBuien by hand as README#install describes: %s\n' "$readme_install"
}

# Downloads the pinned asset into a private temp dir and checks it against the
# pin. Fails, having said why, before anything reaches pacman.
download_and_verify() {
  local url=$repository/releases/download/v${pin_version%-*}/$pin_asset
  tmp=$(mktemp -d)
  printf 'Downloading %s\n' "$url"
  if ! curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
    --tlsv1.2 --output "$tmp/$pin_asset" "$url"; then
    printf 'Could not download %s from the GitHub release, so nothing was installed.\n' "$pin_asset"
    printf 'Install AllesBuien by hand as README#install describes: %s\n' "$readme_install"
    return 1
  fi
  if ! (cd "$tmp" && sha256sum --check --strict "$pin_file"); then
    printf 'The download of %s does not match release.sha256, so nothing was installed.\n' "$pin_asset"
    printf 'Install AllesBuien by hand as README#install describes: %s\n' "$readme_install"
    return 1
  fi
}

install_verb() {
  local running=0
  trap 'if [[ -n $tmp ]]; then rm -rf -- "$tmp"; fi' EXIT
  decide
  case $decision in
    0)
      printf 'AllesBuien is already installed and there is nothing for this click to install. Opening it.\n'
      launch_detached
      return 0
      ;;
    11)
      no_pin_message
      return 1
      ;;
    10)
      printf 'Installing AllesBuien %s from the GitHub release.\n' "$pin_version"
      if ! download_and_verify; then
        return 1
      fi
      if ! sudo pacman -U "$tmp/$pin_asset"; then
        printf 'Nothing was installed. Install AllesBuien by hand as README#install describes: %s\n' "$readme_install"
        return 1
      fi
      launch_detached
      return 0
      ;;
    12)
      if pgrep -x allesbuien >/dev/null; then
        running=1
      fi
      printf 'Updating AllesBuien from %s to %s.\n' "$installed_version" "$pin_version"
      # The app is installed and works, so the click ends with it open whatever
      # happens to the update: a failed download, a declined prompt, or success.
      if ! download_and_verify; then
        printf 'Opening the installed AllesBuien %s.\n' "$installed_version"
        launch_detached
        return 1
      fi
      if sudo pacman -U "$tmp/$pin_asset"; then
        if ((running)); then
          printf 'AllesBuien is running; restart it to use %s.\n' "$pin_version"
        fi
      else
        printf 'Nothing was updated; AllesBuien stays at %s.\n' "$installed_version"
      fi
      launch_detached
      return 0
      ;;
  esac
}

case ${1-} in
  probe)
    decide
    exit "$decision"
    ;;
  launch)
    launch_verb
    ;;
  install)
    install_verb
    ;;
  *)
    printf 'usage: %s probe|launch|install\n' "${0##*/}" >&2
    exit 64
    ;;
esac
