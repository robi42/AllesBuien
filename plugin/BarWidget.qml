// The Omarchy bar launcher for AllesBuien. It opens, focuses, or installs the
// native app and does nothing else: no weather is fetched, parsed, or drawn
// here, and no colour is named - the button takes its font, size, and palette
// from the bar it is placed in.
import QtQuick
import Quickshell.Io
import qs.Ui

BarWidget {
  id: root
  moduleName: "io.github.robi42.allesbuien"

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  // Whether a probe has ever finished, and what it found. Before the first one
  // returns the button looks ordinary and a click simply waits: a dimmed icon
  // on an installed machine would be a lie told for a few milliseconds.
  property bool known: false
  property bool installed: false
  property bool pendingClick: false

  // Focus the open window if there is one, else launch, in the command shape
  // Omarchy's launch_sole helper builds (its Obsidian binding: "^obsidian$").
  // The helper tests the pattern against every window's class and title, any
  // case, so a bare word would also focus a terminal or browser whose title
  // mentions AllesBuien; anchored, only a class or whole title of AllesBuien
  // answers. uwsm-app gives the app its own scope, as the install branch does.
  readonly property string launchCommand: "omarchy-launch-or-focus '^allesbuien$' 'uwsm-app -- allesbuien'"

  // The body of omarchy-install-and-launch with the AUR helper in place of the
  // pacman-only one, because allesbuien-bin lives on the AUR. Omarchy's
  // floating terminal is where yay asks for the password, so the prompt is
  // visible rather than hidden behind a bar click. The last clause is for the
  // reader who arrives before the AUR package exists, or without a network.
  readonly property string installCommand: "omarchy-launch-floating-terminal-with-presentation \"echo 'Installing AllesBuien from the AUR...'; omarchy-pkg-aur-add allesbuien-bin && (setsid uwsm-app -- allesbuien >/dev/null 2>&1 &) || echo 'Could not install allesbuien-bin. Install it by hand: https://github.com/robi42/AllesBuien#install'\""

  function dispatch(): void {
    if (!root.bar) return

    root.bar.run(root.installed ? root.launchCommand : root.installCommand)
  }

  // bar.run is `bash -lc`, whose PATH carries ~/.local/bin. A probe run through
  // any other shell would miss a user-scope install there and offer to install
  // an app that is already on the machine, so the probe uses the same login
  // shell the launch will.
  Process {
    id: probe
    command: ["bash", "-lc", "command -v allesbuien"]

    onExited: function (exitCode) {
      root.installed = exitCode === 0
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
  // machine that never installs it. The other direction is not polled; the
  // next click catches it.
  Timer {
    interval: 5000
    repeat: true
    running: root.known && !root.installed
    onTriggered: probe.running = true
  }

  Component.onCompleted: probe.running = true

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: "󰐷" // U+F0437, Nerd Font md-radar: a dish, never a weather condition -
              // the Omarchy weather pill beside it already draws the sky.
    dimmed: root.known && !root.installed
    tooltipText: root.known && !root.installed ? "AllesBuien is not installed — click to install it from the AUR" : "AllesBuien"
    onPressed: function () {
      root.pendingClick = true
      probe.running = true
    }
  }
}
