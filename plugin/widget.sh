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
#                         11  absent, and nothing this widget can install: no
#                             usable pin, or an allesbuien package that pacman
#                             records with no allesbuien on PATH; a click says
#                             why, and where to read about installing by hand
#                         12  ours and older than the pin: a click updates
#                       Any other code is this script failing: the widget
#                       dims the icon and a click opens the floating terminal,
#                       as for 11, where the failure can be read, but the
#                       tooltip says the check failed and no polling starts.
#   widget.sh launch    focus the open window, or start the app
#   widget.sh install   what Omarchy's floating terminal runs for 10, 11, 12
#                       and a failed probe: decide again, then download,
#                       verify, check the package's Qt against this machine's,
#                       and hand the package to pacman, whose own prompt is
#                       the consent
#
# The pin is one line in `sha256sum --check` format: a SHA-256 digest, two
# spaces, and the release asset's name, allesbuien-<version>-<pkgrel>-x86_64
# .pkg.tar.zst. The version, the tag, and the download URL all come from that
# name, and the digest exists nowhere else. The pin is written after a release's
# asset exists, so between a version bump and that commit it names the previous
# release. The widget never asks GitHub which release is the latest: what a
# click installs is what this plugin checkout pins, and
# `omarchy plugin update` is the only way a newer pin arrives.

readonly repository=https://github.com/robi42/AllesBuien
readonly readme_install=$repository#install
readonly readme_uninstall=$repository#uninstall

# Absolute, so the pin can still be named after the verify step changes into
# the temp dir, and so the detached launch finds this script from anywhere.
self=$(readlink -f -- "${BASH_SOURCE[0]}")
readonly self
readonly pin_file=${self%/*}/release.sha256

# Filled by read_pin and decide.
pin_digest=
pin_asset=
pin_version=
pin_problem=
installed_version=
stray_package=
decision=
tmp=

# Sets pin_digest, pin_asset and pin_version from a usable pin and succeeds; fails when
# there is none. A pin that exists but cannot be read, or does not have the one
# line's shape, is no pin at all - installing what a malformed line seems to
# name would be guessing - and pin_problem says which, for the person reading
# the terminal and on stderr for whoever reads the probe's.
read_pin() {
  local contents pattern
  pin_digest=
  pin_asset=
  pin_version=
  pin_problem=
  if [[ ! -e $pin_file ]]; then
    return 1
  fi
  if [[ ! -f $pin_file || ! -r $pin_file ]] || ! contents=$(<"$pin_file"); then
    pin_problem="$pin_file cannot be read"
  else
    pattern='^([0-9a-f]{64})  (allesbuien-([^-]+-[0-9]+)-x86_64\.pkg\.tar\.zst)$'
    if [[ $contents != *$'\n'* && $contents =~ $pattern ]]; then
      pin_digest=${BASH_REMATCH[1]}
      pin_asset=${BASH_REMATCH[2]}
      pin_version=${BASH_REMATCH[3]}
      return 0
    fi
    pin_problem="$pin_file is not one sha256sum line naming an allesbuien package: ${contents%%$'\n'*}"
  fi
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
  if ! resolved=$(readlink -f "$1"); then
    return 1
  fi
  grep -qxF -- "$resolved" <<<"$files"
}

# Sets decision to one of the probe's exit codes, and installed_version when
# the binary is ours. Only an installed version older than the pin by vercmp,
# pkgrel included, is an update; equal or newer is left as it is, so a click
# never downgrades. Fails, leaving decision as it was, when pacman cannot
# report the installed version, because an unknown version must never count as
# an older one: vercmp ranks an empty version below every other. Each step
# checks its own failure rather than leaving it to set -e, which bash suspends
# when decide runs inside a condition, as it does once a download is done.
#
# No allesbuien on PATH is a fresh install only if pacman records no
# allesbuien package either. A package whose binary is gone - deleted by hand,
# or kept off PATH - may be newer than the pin, so installing the pin over it
# could downgrade it, and nothing on PATH says whose it is: stray_package holds
# its version, and the click installs nothing and says why.
decide() {
  local binary line order
  installed_version=
  stray_package=
  if ! binary=$(command -v allesbuien); then
    if line=$(pacman -Q allesbuien 2>/dev/null); then
      stray_package=${line#allesbuien }
      decision=11
    elif read_pin; then
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
  if ! line=$(pacman -Q allesbuien); then
    return 1
  fi
  installed_version=${line#allesbuien }
  if [[ -z $installed_version || $installed_version == "$line" ]]; then
    printf 'widget.sh: pacman -Q allesbuien answered "%s"\n' "$line" >&2
    return 1
  fi
  if ! order=$(vercmp "$installed_version" "$pin_version"); then
    return 1
  fi
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

# The messages below are read in Omarchy's floating terminal, a window 875
# pixels wide whose terminal breaks a longer line wherever its edge falls,
# mid-word or mid-URL. So a line of prose stays within 72 columns, and a URL, a
# path, or a pin quoted back gets a line of its own, indented by two spaces.
# tests/widget/run.sh holds every click to that.

# Where a click that installs nothing points instead.
by_hand_message() {
  printf 'Install AllesBuien by hand as README#install describes:\n  %s\n' "$readme_install"
}

no_pin_message() {
  if [[ -n $pin_problem ]]; then
    printf 'This copy of the AllesBuien plugin carries a release pin it cannot use:\n  %s\n' "$pin_problem"
    printf 'That is a defect in the plugin, not on this machine,\nso nothing was installed.\n'
  else
    printf 'This copy of the AllesBuien plugin pins no release yet,\nso there is nothing to install from here.\n'
  fi
  by_hand_message
}

unreadable_message() {
  printf 'Could not read which AllesBuien is installed, so nothing was installed.\n'
  printf "pacman's own message above says why.\n"
}

stray_package_message() {
  printf 'pacman records the allesbuien package, version %s,\n' "$stray_package"
  printf 'but no allesbuien command is on PATH, so this click cannot tell\n'
  printf 'what it would replace and installs nothing.\n'
  printf 'Reinstall it as README#install describes:\n  %s\n' "$readme_install"
  printf 'Or remove it as README#uninstall describes:\n  %s\n' "$readme_uninstall"
}

# Downloads the pinned asset into a private temp dir. Fails, having said why,
# before anything reaches pacman.
# The terminal names the release's page rather than the asset's own URL, which
# is too long for one line of it.
download_pinned() {
  local tag=v${pin_version%-*}
  local url=$repository/releases/download/$tag/$pin_asset
  if ! tmp=$(mktemp -d); then
    printf 'Could not create a temporary directory, so nothing was installed.\n'
    return 1
  fi
  printf 'Downloading %s from\n  %s\n' "$pin_asset" "$repository/releases/tag/$tag"
  if ! curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
    --tlsv1.2 --output "$tmp/$pin_asset" "$url"; then
    printf 'Could not download %s\nfrom the GitHub release, so nothing was installed.\n' "$pin_asset"
    by_hand_message
    return 1
  fi
}

# Checks the download against the pin. Fails, having said why, before anything
# reaches pacman.
verify_download() {
  if ! (cd "$tmp" && sha256sum --check --strict "$pin_file"); then
    printf 'The download of %s\ndoes not match release.sha256, so nothing was installed.\n' "$pin_asset"
    by_hand_message
    return 1
  fi
}

# Checks the download against this machine's Qt. The app compiles its QML
# ahead of time, so its binary calls into Qt's private API, which no Qt release
# promises to keep: a build against Qt 6.12 needs a function that 6.11 lacks,
# and cannot start there. The package's depends are unversioned, so pacman
# would install it anyway, and a versioned one would do harm: with a fresh sync
# database, pacman -U would satisfy it by upgrading Qt alone, a partial
# upgrade, underneath the running shell. So the click goes on only if the
# installed qt6-declarative is the one the package's .BUILDINFO records, or
# newer, by vercmp of the upstream versions alone, since a pkgrel only rebuilds
# the same Qt release. bsdtar, which comes with the libarchive pacman needs,
# reads the archive only once it has passed the pin's check. Fails, having
# said why, before sudo asks for anything: when the installed Qt is older, when
# either version cannot be read, and when vercmp fails or answers anything but
# -1, 0 or 1, because an unknown version or order must never count as a new
# enough one.
qt_new_enough() {
  local buildinfo line installed order pattern built=
  pattern='^installed = qt6-declarative-([0-9][^-]*)-[^-]+-[^-]+$'
  if buildinfo=$(bsdtar -xOf "$tmp/$pin_asset" .BUILDINFO); then
    while IFS= read -r line; do
      if [[ $line =~ $pattern ]]; then
        built=${BASH_REMATCH[1]}
        break
      fi
    done <<<"$buildinfo"
  fi
  if [[ -z $built ]]; then
    printf 'Could not read which Qt AllesBuien %s was built against,\nso nothing was installed.\n' "$pin_version"
    by_hand_message
    return 1
  fi
  if ! line=$(pacman -Q qt6-declarative) || [[ $line != "qt6-declarative "?* ]]; then
    printf 'Could not read which Qt is installed, so nothing was installed.\n'
    return 1
  fi
  installed=${line#qt6-declarative }
  installed=${installed%-*}
  if ! order=$(vercmp "$installed" "$built"); then
    order=
  fi
  case $order in
    0 | 1)
      return 0
      ;;
    -1)
      printf 'AllesBuien %s needs Qt %s or newer, and this machine\n' "$pin_version" "$built"
      printf 'has Qt %s, so nothing was installed.\n' "$installed"
      printf 'Update Omarchy with omarchy-update, then click again.\n'
      ;;
    *)
      printf 'Could not compare Qt %s with Qt %s,\nso nothing was installed.\n' "$installed" "$built"
      ;;
  esac
  return 1
}

# Takes sudo's password now, with its -v, rather than when pacman starts. The
# prompt waits as long as the person does, and the last decision has to come
# after every such wait; sudo then reuses the credential for pacman a moment
# later. Fails, having said so, if sudo is not satisfied.
authorise() {
  if ! sudo -v; then
    printf 'sudo did not authorise pacman, so nothing was installed.\n'
    return 1
  fi
}

# The download, the check, and the password prompt take time, and meanwhile a
# second click, another installer, or a plugin update can change what is
# installed or what is pinned: a click acting on the answer it started with
# could then reinstall the pinned release over itself, or downgrade an install
# that has moved past the pin. So the decision is taken again last, and pacman
# gets the file only if the answer still holds, for the same pin line. Fails,
# having said so, otherwise.
still_wanted() {
  local before=$decision asset=$pin_asset digest=$pin_digest
  if ! decide; then
    unreadable_message
    return 1
  fi
  if [[ $decision == "$before" && $pin_asset == "$asset" && $pin_digest == "$digest" ]]; then
    return 0
  fi
  printf 'AllesBuien changed since this click began, so nothing was installed.\n'
  return 1
}

# Everything between a decision to install and pacman, each step stopping the
# click if it fails, the last decision last. From the moment pacman starts it
# holds its database lock, through its own confirmation prompt until it
# commits, so no other install can land after that decision but in the instant
# before pacman takes the lock. pacman has no flag that refuses a downgrade;
# --needed on the install skips a package that has become the pinned version
# in that instant.
prepare_install() {
  download_pinned && verify_download && qt_new_enough && authorise && still_wanted
}

# After a click that installed nothing, the app opens if it is installed: as it
# was for an update that failed, or as it has become since the click began.
launch_if_installed() {
  if [[ $decision == 0 || $decision == 12 ]]; then
    printf 'Opening AllesBuien.\n'
    launch_detached
  fi
}

install_verb() {
  local running=0
  trap 'if [[ -n $tmp ]]; then rm -rf -- "$tmp"; fi' EXIT
  if ! decide; then
    unreadable_message
    return 1
  fi
  case $decision in
    0)
      printf 'AllesBuien is already installed and there is nothing\nfor this click to install. Opening it.\n'
      launch_detached
      return 0
      ;;
    11)
      if [[ -n $stray_package ]]; then
        stray_package_message
      else
        no_pin_message
      fi
      return 1
      ;;
    10)
      printf 'Installing AllesBuien %s from the GitHub release.\n' "$pin_version"
      if ! prepare_install; then
        launch_if_installed
        return 1
      fi
      if ! sudo pacman -U --needed "$tmp/$pin_asset"; then
        printf 'Nothing was installed.\n'
        by_hand_message
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
      if ! prepare_install; then
        launch_if_installed
        return 1
      fi
      if sudo pacman -U --needed "$tmp/$pin_asset"; then
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
    if ! decide; then
      exit 1
    fi
    if [[ -n $pin_problem ]]; then
      printf 'widget.sh: %s\n' "$pin_problem" >&2
    fi
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
