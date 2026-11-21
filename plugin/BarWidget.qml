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

  // The two intervals the schedule below runs on, five seconds between probes
  // and the ten minutes a click is followed for. Nothing in the shell sets
  // them; they are properties so that tests/widget/qml/tst_schedule.qml can
  // shorten them, and so the timers name no number of their own.
  property int pollInterval: 5000
  property int followUpWindow: 600000

  // Whether a click that opened the floating terminal is still being followed:
  // its window is open and the last probe did not answer 0. A 0 says the
  // terminal has nothing left to do, so it ends the follow-up at once.
  readonly property bool following: followUp.running && root.probeStatus !== 0

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

    const launching = root.probeStatus === 0
    root.bar.run(launching ? root.launchCommand : root.installCommand)
    if (!launching) root.broadcast("follow")
  }

  // Opens, or reopens, the follow-up window on this copy of the widget. A click
  // that opened the terminal reaches it through `broadcast`, which Omarchy's
  // bar answers by calling it on every monitor's copy, the clicked one
  // included, so the icon clears on each of them and not only where it was
  // clicked.
  function follow(): void {
    followUp.restart()
  }

  // The follow-up window: ten minutes from the last click that opened the
  // floating terminal. bar.run starts that terminal detached and hands nothing
  // back, and its password and pacman prompts wait as long as the person does,
  // so the widget cannot know when it ends and bounds its interest instead.
  //
  // A probe that answers 0 ends the follow-up, as `following` says; one that
  // answers anything else, a changed answer included, does not, because the
  // answer can change twice before the terminal is done. A probe that fails
  // while an update is under way goes 12, a failure, 0, and would leave the
  // icon dimmed with the failed-check tooltip. A click on a failed probe opens
  // a terminal that decides again and updates, the failure, 12, 0, and an
  // install click whose terminal finds an older package and updates it goes
  // 10, 12, 0; both would leave the icon urgent after the update. The cost is
  // that an answer that changes to anything but 0 is probed until the window
  // closes.
  //
  // The five-second ticks need not meet the window's end, so the last one can
  // fall up to five seconds before it. As the window closes, the widget probes
  // once more unless the last answer was 0, so an update that lands after
  // that tick still clears the icon.
  Timer {
    id: followUp
    interval: root.followUpWindow
    onTriggered: if (root.probeStatus !== 0) probe.running = true
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

  // Probes every five seconds on two occasions. While the app is known to be
  // absent, for as long as that lasts, which on a machine that never installs
  // it is for good: that is the price of the icon lighting within five seconds
  // of an install finishing elsewhere. And while a click that opened the
  // floating terminal is followed, until a probe answers 0 or ten minutes
  // pass, with one last probe as they end, so the icon leaves the urgent
  // colour within five seconds of an update finishing, with no click. An
  // installed app is otherwise never polled, and a failed probe alone starts
  // no polling; the next click or load probes again. When the terminal is
  // done only after the ten minutes and that last probe, the icon keeps its
  // look until the next click, which probes and catches up. A newer pin
  // arrives through `omarchy plugin update`, whose rescan recreates this
  // widget, and the load-time probe is what turns the icon urgent.
  Timer {
    interval: root.pollInterval
    repeat: true
    running: root.known && (root.absent || root.following)
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
