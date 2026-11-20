import QtQml
import Script

// Stands in for Quickshell's Process. Setting running starts it, as with the
// real one, and it exits on a later turn of the event loop with whatever the
// fake desktop answers then. It counts its starts, which is how the check
// tells a widget that probes from one that does not.
QtObject {
  id: process
  property var command: []
  property bool running: false
  property int starts: 0
  signal exited(int exitCode, int exitStatus)

  onRunningChanged: {
    if (!running) return
    starts += 1
    Qt.callLater(function () {
      process.running = false
      process.exited(Desktop.answer, 0)
    })
  }
}
