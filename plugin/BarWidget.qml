// The Omarchy bar launcher for AllesBuien. It opens or focuses the native app,
// and installs or updates the release that release.sha256 pins, and does
// nothing else: no weather is fetched, parsed, or drawn here, and no colour is
// named - the button takes its font, size, and palette from the bar it is
// placed in. Every decision and every install step lives in widget.sh beside
// this file; this file maps that script's exit codes onto the button.
import QtQuick
import Quickshell.Io
import qs.Commons
import qs.Ui

BarWidget {
  id: root
  moduleName: "io.github.robi42.allesbuien"

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  // Whether a probe has ever finished, and what it answered. Before the first
  // one returns the button looks ordinary and a click simply waits: a dimmed
  // icon on an installed machine would be a lie told for a few milliseconds.
  //
  // The answer is widget.sh probe's exit code, and the contract is stated once,
  // there: 0 launch or focus; 10 absent with a release pinned, so a click
  // installs it; 11 absent with nothing it can install, so a click says why
  // and where to read about installing by hand; 12 ours and older than the pin,
  // so a click updates. Any other code is the script failing: the icon dims
  // and the click opens a terminal where the failure can be read, as for 11,
  // but the tooltip says the check failed. It is not taken for absence, because
  // it says nothing about whether the app is there: a probe whose pacman query
  // failed has not found out.
  property bool known: false
  property int probeStatus: 0
  property bool pendingClick: false

  readonly property bool installed: root.probeStatus === 0 || root.probeStatus === 12
  readonly property bool absent: root.probeStatus === 10 || root.probeStatus === 11
  readonly property bool updateReady: root.probeStatus === 12
  readonly property bool installable: root.probeStatus === 10

  // widget.sh sits beside this file wherever the shell loaded the plugin from,
  // so its path comes from this file's own URL rather than from a plugins
  // directory written down here.
  readonly property string scriptPath: decodeURIComponent(String(Qt.resolvedUrl("widget.sh")).replace(/^file:\/\//, ""))

  // bar.run hands its string to `bash -lc`, so the path is quoted for that
  // shell. The install command is quoted twice: once for bar.run's shell, and
  // once for the `bash -c` into which Omarchy's floating-terminal wrapper
  // splices its arguments. It runs the script as a login shell there too, since
  // the terminal inherits the app launcher's environment rather than the one
  // the probe saw. Omarchy's floating terminal is where pacman asks for the
  // password, so the prompt is visible rather than hidden behind a bar click.
  readonly property string launchCommand: "bash " + Util.shellQuote(root.scriptPath) + " launch"
  readonly property string installCommand: "omarchy-launch-floating-terminal-with-presentation "
    + Util.shellQuote("bash -l " + Util.shellQuote(root.scriptPath) + " install")

  function dispatch(): void {
    if (!root.bar) return

    root.bar.run(root.probeStatus === 0 ? root.launchCommand : root.installCommand)
  }

  // bar.run is `bash -lc`, whose PATH carries ~/.local/bin. A probe run through
  // any other shell would miss a user-scope install there and offer to install
  // an app that is already on the machine, so the probe uses the same login
  // shell the launch will. The path travels as $0, never re-tokenised.
  Process {
    id: probe
    command: ["bash", "-lc", "exec bash \"$0\" probe", root.scriptPath]

    onExited: function (exitCode) {
      root.probeStatus = exitCode
      root.known = true

      // A click always acts on a probe that finished after the press, never on
      // a remembered answer: install the app while the icon is still dimmed,
      // and the click that follows launches it.
      if (root.pendingClick) {
        root.pendingClick = false
        root.dispatch()
      }
    }
  }

  // Only while the app is known to be missing, so the icon lights within five
  // seconds of an install finishing elsewhere and nothing polls forever on a
  // machine that never installs it. A failed probe is not known absence, so it
  // starts no polling either; the next click or load probes again. The other
  // direction is not polled; the next click catches it. A newer pin arrives
  // through `omarchy plugin update`, whose rescan recreates this widget, and the
  // load-time probe is what turns the icon urgent; after an update it stays so
  // until the next click, which probes, finds the app current, and launches it.
  Timer {
    interval: 5000
    repeat: true
    running: root.known && root.absent
    onTriggered: probe.running = true
  }

  Component.onCompleted: probe.running = true

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: "󰐷" // U+F0437, Nerd Font nf-md-radar: a radar screen, never a
              // weather condition, because the Omarchy weather pill beside it
              // already draws the sky.
    dimmed: root.known && !root.installed
    // The bar's urgent colour, as Omarchy's own update indicator uses it.
    active: root.known && root.updateReady
    tooltipText: !root.known || root.probeStatus === 0 ? "AllesBuien"
      : root.updateReady ? "AllesBuien update ready — click to install it"
      : root.installable ? "AllesBuien is not installed — click to install it from the GitHub release"
      : root.absent ? "AllesBuien is not installed — see github.com/robi42/AllesBuien#install"
      : "AllesBuien could not check its installation — click to see why"
    onPressed: function () {
      root.pendingClick = true
      probe.running = true
    }
  }
}
