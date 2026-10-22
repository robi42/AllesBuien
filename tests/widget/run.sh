#!/usr/bin/env bash
set -euo pipefail

# The proof of plugin/widget.sh, without Omarchy, a network, or root. Each case
# runs a fresh copy of the script, with or without a release.sha256 beside it,
# against the fakes in tests/widget/fakes: pacman, vercmp, curl, sha256sum,
# sudo, pgrep, and omarchy-launch-or-focus. Each fake appends the command line
# it was given to a per-case log, and the cases assert on that log and on the
# script's exit code and output.
#
# The script sees a PATH built from nothing: the fakes, a directory of links to
# the handful of real tools the script and the fakes use, and an allesbuien
# stub only where a case wants the app installed. The host's PATH would bring
# the host's own allesbuien with it - a Mode B install puts one in
# ~/.local/bin - and turn every "absent" case into an installed one, and a
# fake left out would fall through to the real sudo or pacman instead of
# failing. The runner keeps the host PATH for its own work and hands the built
# one to the script alone, with no BASH_ENV, so no startup file can put the
# host's back.
#
# The install verb starts the final launch detached, as it must in a real
# terminal, so the launch can still be on its way when the script exits. Each
# run therefore holds a spare descriptor, 9, open on the pipe that captures the
# output: the detached launch inherits it, and the capture only ends when every
# process holding it has exited. A case that expects no launch is proven by the
# same wait, not by a pause long enough to hope the launch would have shown.

here=$(dirname -- "$(readlink -f -- "${BASH_SOURCE[0]}")")
widget=$(readlink -f -- "$here/../../plugin/widget.sh")
fakes=$here/fakes

readonly digest=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
readonly asset=allesbuien-0.2.0-1-x86_64.pkg.tar.zst
readonly url=https://github.com/robi42/AllesBuien/releases/download/v0.2.0/$asset

scratch=$(mktemp -d)
scratch=$(readlink -f -- "$scratch")
trap 'rm -rf -- "$scratch"' EXIT

mkdir "$scratch/bin"
for tool in bash chmod mktemp rm grep readlink sort; do
  if ! real=$(command -v "$tool"); then
    printf 'FAIL the runner needs %s, and it is not on PATH\n' "$tool" >&2
    exit 1
  fi
  ln -s -- "$real" "$scratch/bin/$tool"
done

failures=0
cases=0

# Per-case state: the case's name and directory, the first problem found, the
# fakes' settings, and the directories that hold an allesbuien.
name=
case_dir=
problem=
env_vars=()
app_path=
output=
status=
verb=

begin() {
  name=$1
  problem=
  verb=
  case_dir=$scratch/$name
  mkdir -p "$case_dir/plugin" "$case_dir/tmp" "$case_dir/state"
  cp -- "$widget" "$case_dir/plugin/widget.sh"
  : >"$case_dir/log"
  app_path=
  env_vars=(
    "FAKE_LOG=$case_dir/log"
    "FAKE_STATE=$case_dir/state"
    "TMPDIR=$case_dir/tmp"
    FAKE_PACKAGE=absent
    FAKE_PACKAGE_FILES=
    FAKE_CURL=ok
    "FAKE_DOWNLOAD=$digest"
    FAKE_PACMAN_U=ok
    FAKE_RUNNING=0
  )
}

# Later settings win, as env applies them in order.
set_env() {
  env_vars+=("$@")
}

pin() {
  printf '%s  allesbuien-%s-x86_64.pkg.tar.zst\n' "$digest" "$1" >"$case_dir/plugin/release.sha256"
}

# An allesbuien in a directory of the case's own, appended to the script's
# PATH, so a directory added first is found first.
app_at() {
  local dir=$case_dir/$1
  mkdir -p "$dir"
  printf '#!/usr/bin/env bash\n' >"$dir/allesbuien"
  chmod +x "$dir/allesbuien"
  app_path+=${app_path:+:}$dir
}

# The allesbuien pacman package at version $1, owning the binary in usr-bin.
ours() {
  app_at usr-bin
  set_env "FAKE_PACKAGE=$1" "FAKE_PACKAGE_FILES=$(readlink -f -- "$case_dir/usr-bin/allesbuien")"
}

run() {
  verb=$1
  if output=$(env -u BASH_ENV -u ENV "${env_vars[@]}" "PATH=$fakes:$scratch/bin${app_path:+:$app_path}" \
    bash "$case_dir/plugin/widget.sh" "$1" 2>&1 9>&1); then
    status=0
  else
    status=$?
  fi
}

note() {
  if [[ -z $problem ]]; then
    problem=$1
  fi
}

expect_status() {
  if [[ $status -ne $1 ]]; then
    note "exit $status, expected $1"
  fi
}

expect_output() {
  if [[ $output != *"$1"* ]]; then
    note "the output never says: $1"
  fi
}

expect_no_output() {
  if [[ $output == *"$1"* ]]; then
    note "the output says: $1"
  fi
}

# The case's log with its own directory written <case> and the script's temp
# dir written <tmp>, so expected calls can be spelled out exactly.
normalized_log() {
  local log
  log=$(<"$case_dir/log")
  log=${log//"$case_dir"/<case>}
  sed -E 's|<case>/tmp/tmp\.[A-Za-z0-9]+|<tmp>|g' <<<"$log"
}

# Each given line appears in the log, in this order, with anything between.
expect_calls() {
  local normalized wanted i=0
  local -a lines
  normalized=$(normalized_log)
  mapfile -t lines <<<"$normalized"
  for wanted in "$@"; do
    while ((i < ${#lines[@]})) && [[ ${lines[i]} != "$wanted" ]]; do
      i=$((i + 1))
    done
    if ((i == ${#lines[@]})); then
      note "no call, in order: $wanted"
      return 0
    fi
    i=$((i + 1))
  done
}

expect_no_call() {
  local normalized line
  normalized=$(normalized_log)
  while IFS= read -r line; do
    if [[ $line == "$1"* ]]; then
      note "unexpected call: $line"
      return 0
    fi
  done <<<"$normalized"
}

# Checked after every case, whatever it is about: pacman never receives a file
# unless the same run's sha256sum check passed on that file first. A check that
# ran and printed FAILED does not count.
pacman_only_after_a_passing_check() {
  local line path verified=" "
  while IFS= read -r line; do
    case $line in
      "sha256sum-verdict "*" OK")
        verified+="${line#sha256sum-verdict } "
        ;;
      "pacman [-U] "*)
        path=${line##*[}
        path=${path%]}
        if [[ $verified != *" ${path##*/} OK "* ]]; then
          note "pacman -U got ${path##*/} without a passing sha256sum check before it"
        fi
        ;;
    esac
  done <"$case_dir/log"
}

# And the script's temp dir is gone once it has exited, download or not.
temp_dir_removed() {
  local -a left
  left=("$case_dir"/tmp/*)
  if [[ -e ${left[0]} ]]; then
    note "the script left ${left[0]#"$case_dir/"} behind"
  fi
}

# And a click's messages fit the terminal they are read in: Omarchy's floating
# terminal, a window 875 pixels wide, breaks a longer line wherever its edge
# falls, mid-word or mid-URL. 72 columns fits that window with room for a
# larger font. A URL, path or pin quoted on a line of its own, indented by two
# spaces, is exempt, since breaking it would be worse. The probe and the launch
# verb print nothing a person reads.
lines_fit_the_terminal() {
  local line
  if [[ $verb != install ]]; then
    return 0
  fi
  while IFS= read -r line; do
    if [[ $line != "  "* ]] && ((${#line} > 72)); then
      note "a line of ${#line} columns, wider than the terminal's 72: $line"
      return 0
    fi
  done <<<"$output"
}

finish() {
  pacman_only_after_a_passing_check
  temp_dir_removed
  lines_fit_the_terminal
  cases=$((cases + 1))
  if [[ -n $problem ]]; then
    failures=$((failures + 1))
    printf 'FAIL %s: %s\n' "$name" "$problem"
    if [[ -n $output ]]; then
      printf '%s\n' "$output" | sed 's/^/    /'
    fi
  else
    printf 'ok %s\n' "$name"
  fi
}

readonly launch_call="omarchy-launch-or-focus [^allesbuien\$] [uwsm-app -- allesbuien]"
readonly curl_call="curl [--fail] [--silent] [--show-error] [--location] [--proto] [=https] [--proto-redir] [=https] [--tlsv1.2] [--output] [<tmp>/$asset] [$url]"
readonly check_call="sha256sum [--check] [--strict] [<case>/plugin/release.sha256]"
readonly auth_call="sudo [-v]"
readonly sudo_call="sudo [pacman] [-U] [--needed] [<tmp>/$asset]"
readonly pacman_call="pacman [-U] [--needed] [<tmp>/$asset]"

# The probe: one exit code per state.

begin probe-absent-with-pin
pin 0.2.0-1
run probe
expect_status 10
finish

begin probe-absent-no-pin
run probe
expect_status 11
finish

begin probe-absent-malformed-pin
printf '%s  %s\n' "${digest:1}" "$asset" >"$case_dir/plugin/release.sha256"
run probe
expect_status 11
expect_output "is not one sha256sum line naming an allesbuien package: ${digest:1}  $asset"
finish

begin probe-update-older-version
pin 0.2.0-1
ours 0.1.0-1
run probe
expect_status 12
expect_calls "vercmp [0.1.0-1] [0.2.0-1]"
finish

begin probe-update-older-pkgrel
pin 0.2.0-2
ours 0.2.0-1
run probe
expect_status 12
finish

begin probe-current-equal
pin 0.2.0-1
ours 0.2.0-1
run probe
expect_status 0
finish

begin probe-current-newer-than-pin
pin 0.2.0-1
ours 0.3.0-1
run probe
expect_status 0
finish

# A package other than allesbuien owns the binary, as an allesbuien-bin from
# the AUR would: the allesbuien package itself is not installed.
begin probe-foreign-owner
pin 0.2.0-1
app_at usr-bin
run probe
expect_status 0
finish

begin probe-unowned-mode-b
pin 0.2.0-1
app_at local-bin
run probe
expect_status 0
finish

# The package is installed and older than the pin, but PATH finds a Mode B
# binary first, and that binary is what the widget launches: not ours.
begin probe-mode-b-shadows-package
pin 0.2.0-1
app_at local-bin
ours 0.1.0-1
run probe
expect_status 0
finish

# A link to the package's own binary is still the package's binary.
begin probe-symlink-to-package-file
pin 0.2.0-1
mkdir -p "$case_dir/link-bin"
app_path=$case_dir/link-bin
ours 0.1.0-1
ln -s -- "$case_dir/usr-bin/allesbuien" "$case_dir/link-bin/allesbuien"
run probe
expect_status 12
finish

begin probe-installed-no-pin
ours 0.1.0-1
run probe
expect_status 0
finish

# Launching.

begin launch-verb
run launch
expect_status 0
expect_calls "$launch_call"
finish

# Installing: absent, and a release pinned.

begin install-happy
pin 0.2.0-1
run install
expect_status 0
expect_calls "$curl_call" "$check_call" "sha256sum-verdict $asset OK" "$auth_call" "$sudo_call" "$pacman_call" "$launch_call"
finish

begin install-download-failure
pin 0.2.0-1
set_env FAKE_CURL=fail
run install
expect_status 1
expect_calls "$curl_call"
expect_no_call "sha256sum"
expect_no_call "sudo"
expect_no_call "pacman [-U]"
expect_no_call "omarchy-launch-or-focus"
expect_output "Could not download $asset"
expect_output "README#install"
finish

begin install-digest-mismatch
pin 0.2.0-1
set_env FAKE_DOWNLOAD=tampered
run install
expect_status 1
expect_calls "$check_call" "sha256sum-verdict $asset FAILED"
expect_no_call "sudo"
expect_no_call "pacman [-U]"
expect_no_call "omarchy-launch-or-focus"
expect_output "does not match release.sha256"
expect_output "README#install"
finish

begin install-declined
pin 0.2.0-1
set_env FAKE_PACMAN_U=declined
run install
expect_status 1
expect_calls "$pacman_call"
expect_no_call "omarchy-launch-or-focus"
expect_output "Nothing was installed"
expect_output "README#install"
finish

begin install-no-pin
run install
expect_status 1
expect_no_call "curl"
expect_no_call "omarchy-launch-or-focus"
expect_output "pins no release yet"
expect_output "README#install"
finish

begin install-malformed-pin
printf 'not a pin\n' >"$case_dir/plugin/release.sha256"
run install
expect_status 1
expect_no_call "curl"
expect_output "carries a release pin it cannot use"
expect_output "README#install"
finish

# A click that arrives once the app is installed, here by another package,
# installs nothing whatever the widget last showed.
begin install-when-current
pin 0.2.0-1
app_at usr-bin
run install
expect_status 0
expect_no_call "curl"
expect_no_call "pacman [-U]"
expect_calls "$launch_call"
expect_output "already installed"
finish

# Updating: ours, and older than the pin.

begin update-happy
pin 0.2.0-1
ours 0.1.0-1
set_env FAKE_RUNNING=1
run install
expect_status 0
expect_output "Updating AllesBuien from 0.1.0-1 to 0.2.0-1"
expect_calls "$curl_call" "sha256sum-verdict $asset OK" "$pacman_call" "$launch_call"
expect_output "AllesBuien is running; restart it to use 0.2.0-1"
finish

begin update-happy-not-running
pin 0.2.0-1
ours 0.1.0-1
run install
expect_status 0
expect_calls "$pacman_call" "$launch_call"
expect_no_output "restart it"
finish

begin update-declined
pin 0.2.0-1
ours 0.1.0-1
set_env FAKE_RUNNING=1 FAKE_PACMAN_U=declined
run install
expect_status 0
expect_calls "$pacman_call" "$launch_call"
expect_output "Nothing was updated; AllesBuien stays at 0.1.0-1"
expect_no_output "restart it"
finish

begin update-download-failure
pin 0.2.0-1
ours 0.1.0-1
set_env FAKE_RUNNING=1 FAKE_CURL=fail
run install
expect_status 1
expect_no_call "pacman [-U]"
expect_calls "$launch_call"
expect_output "README#install"
expect_no_output "restart it"
finish

# What is installed changes while the package downloads. The click decides
# again before pacman, and installs nothing unless its answer still holds.

# Another installer moves the package past the pin: updating now would
# downgrade it.
begin update-overtaken-by-newer
pin 0.2.0-1
ours 0.1.0-1
set_env FAKE_PACKAGE_AFTER_DOWNLOAD=0.3.0-1
run install
expect_status 1
expect_no_call "pacman [-U]"
expect_calls "$launch_call"
expect_output "AllesBuien changed since this click began, so nothing was installed"
finish

# A second click's update finishes first: this one would reinstall it.
begin update-overtaken-by-another-click
pin 0.2.0-1
ours 0.1.0-1
set_env FAKE_PACKAGE_AFTER_DOWNLOAD=0.2.0-1
run install
expect_status 1
expect_no_call "pacman [-U]"
expect_calls "$launch_call"
expect_output "changed since this click began"
finish

# A second click's install finishes first, on a machine that had no AllesBuien
# when this click began.
begin install-overtaken-by-another-click
pin 0.2.0-1
mkdir -p "$case_dir/usr-bin"
app_path=$case_dir/usr-bin
set_env FAKE_PACKAGE_AFTER_DOWNLOAD=0.2.0-1 \
  "FAKE_PACKAGE_FILES=$case_dir/usr-bin/allesbuien" \
  "FAKE_APP_AFTER_DOWNLOAD=$case_dir/usr-bin/allesbuien"
run install
expect_status 1
expect_no_call "pacman [-U]"
expect_calls "$launch_call"
expect_output "changed since this click began"
finish

# An install lands while sudo's password prompt waits, which can be as long as
# the person likes: the last decision comes after the prompt, so it sees it.
begin update-overtaken-during-password-prompt
pin 0.2.0-1
ours 0.1.0-1
set_env FAKE_PACKAGE_AFTER_AUTH=0.3.0-1
run install
expect_status 1
expect_calls "$auth_call"
expect_no_call "pacman [-U]"
expect_calls "$launch_call"
expect_output "AllesBuien changed since this click began, so nothing was installed"
finish

# sudo is not satisfied: nothing is installed, and an installed app still opens.
begin install-sudo-declined
pin 0.2.0-1
set_env FAKE_SUDO_V=declined
run install
expect_status 1
expect_no_call "pacman [-U]"
expect_no_call "omarchy-launch-or-focus"
expect_output "sudo did not authorise pacman, so nothing was installed"
finish

begin update-sudo-declined
pin 0.2.0-1
ours 0.1.0-1
set_env FAKE_SUDO_V=declined
run install
expect_status 1
expect_no_call "pacman [-U]"
expect_calls "$launch_call"
expect_output "sudo did not authorise pacman"
finish

# The installed version cannot be read. An unknown version is never older than
# the pin, so nothing is installed. The decision taken again after a download
# runs where bash suspends set -e, so these hold only because every step of it
# checks its own failures.

begin probe-query-fails
pin 0.2.0-1
ours 0.1.0-1
printf 'fail\n' >"$case_dir/state/query-fails"
run probe
expect_status 1
expect_output "fake pacman: -Q made to fail"
finish

begin install-query-fails
pin 0.2.0-1
ours 0.1.0-1
printf 'fail\n' >"$case_dir/state/query-fails"
run install
expect_status 1
expect_no_call "curl"
expect_no_call "pacman [-U]"
expect_output "Could not read which AllesBuien is installed, so nothing was installed"
finish

# pacman -Q fails once the download is done.
begin update-query-fails-after-download
pin 0.2.0-1
ours 0.1.0-1
set_env FAKE_QUERY_FAILS_AFTER_DOWNLOAD=1
run install
expect_status 1
expect_no_call "pacman [-U]"
expect_calls "$launch_call"
expect_output "Could not read which AllesBuien is installed, so nothing was installed"
finish

# pacman records an allesbuien package, but no allesbuien is on PATH: its
# binary was deleted, or is kept off PATH. Installing the pin over it could
# downgrade it, and nothing says whose it is, so the click installs nothing.

begin probe-package-without-binary
pin 0.2.0-1
set_env FAKE_PACKAGE=0.3.0-1
run probe
expect_status 11
finish

begin install-package-without-binary
pin 0.2.0-1
set_env FAKE_PACKAGE=0.3.0-1
run install
expect_status 1
expect_no_call "curl"
expect_no_call "pacman [-U]"
expect_no_call "omarchy-launch-or-focus"
expect_output "pacman records the allesbuien package, version 0.3.0-1,"
expect_output "but no allesbuien command is on PATH"
expect_output "README#install"
expect_output "README#uninstall"
finish

# The pin itself. plugin/release.sha256 is what every widget installs from, so
# the repository's own pin is held to two things here: the widget reads it as a
# usable pin, and it names no release newer than app/Cargo.toml's version - a
# release this tree did not build. It may name an older one: the pin is written
# after a release's asset exists, so it trails a version bump until the
# follow-up pin commit, and that previous release is what widgets should go on
# installing. No pin at all is the state before the first pin commit. The
# review is watched refusing a malformed pin and one ahead of the tree, and
# accepting the lag, before it is trusted with the real one.

app_version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$here/../../app/Cargo.toml")
app_version=${app_version%%$'\n'*}
pin_finding=

# Sets pin_finding to what is wrong with the case's pin, or to nothing.
review_pin() {
  local line shape='allesbuien-([^-]+)-[0-9]+-x86_64\.pkg\.tar\.zst$'
  pin_finding=
  run probe
  if [[ $status -ne 10 ]]; then
    pin_finding="the widget cannot use it: ${output:-the probe answered $status}"
    return 0
  fi
  line=$(<"$case_dir/plugin/release.sha256")
  if [[ -z $app_version ]]; then
    pin_finding="app/Cargo.toml has no version to hold it to"
  elif [[ $line =~ $shape ]] && ! sort -V -C <<<"${BASH_REMATCH[1]}"$'\n'"$app_version"; then
    pin_finding="it names ${BASH_REMATCH[1]}, newer than app/Cargo.toml's $app_version"
  fi
}

begin pin-review-refuses-malformed
printf '%s  %s\n' "${digest:1}" "$asset" >"$case_dir/plugin/release.sha256"
review_pin
if [[ $pin_finding != "the widget cannot use it: "*"is not one sha256sum line"* ]]; then
  note "a digest one character short passed the review: ${pin_finding:-no finding}"
fi
finish

begin pin-review-refuses-ahead-of-app
pin 9.9.9-1
review_pin
if [[ $pin_finding != "it names 9.9.9, newer than app/Cargo.toml's $app_version" ]]; then
  note "a pin ahead of app/Cargo.toml passed the review: ${pin_finding:-no finding}"
fi
finish

begin pin-review-allows-lag
pin 0.0.1-1
review_pin
if [[ -n $pin_finding ]]; then
  note "a pin trailing app/Cargo.toml was refused: $pin_finding"
fi
finish

begin committed-pin
if [[ -e $here/../../plugin/release.sha256 ]]; then
  cp -- "$here/../../plugin/release.sha256" "$case_dir/plugin/release.sha256"
  review_pin
  if [[ -n $pin_finding ]]; then
    note "plugin/release.sha256 does not pin a release this tree can install: $pin_finding"
  fi
fi
finish

if ((failures > 0)); then
  printf '%d of %d widget cases failed\n' "$failures" "$cases"
  exit 1
fi
printf 'all %d widget cases passed\n' "$cases"
