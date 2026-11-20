// The check of plugin/BarWidget.qml's probe schedule. It loads the real
// widget under the fake modules in fakes/ for qs.Commons, qs.Ui and
// Quickshell.Io, which exist only on an Omarchy desktop, and shortens the
// widget's two intervals, 5 s and ten minutes, to 20 ms and 600 ms through the
// widget's own pollInterval and followUpWindow. The third test in
// tests/widget/tests/scripts.rs runs it inside `cargo test --workspace`; to
// run it alone, from the repository root:
//
//   QT_QPA_PLATFORM=offscreen /usr/lib/qt6/bin/qmltestrunner \
//     -import tests/widget/qml/fakes -input tests/widget/qml/tst_schedule.qml
//
// It proves the widget's own half: which probes it starts, when it stops, and
// that a click asks every copy to follow. That the real bar delivers the relay
// to the copies on other monitors is Omarchy's half, which only a real bar
// shows.
//
// Every wait is a condition with a budget: the widget's own `following` says
// when a follow-up has ended, so no case infers it from a pause. A fixed wait
// appears in two places only: quiet(), whose claim is that nothing happens,
// and the case that lets half a window pass before a second click. A stall
// can only make a measured window longer, so the bound cases cannot fail for
// a slow machine.
import QtQuick
import QtTest
import Script

TestCase {
  id: suite
  name: "BarWidgetSchedule"

  readonly property int poll: 20
  readonly property int window: 600
  // What a window may fall short of its length by, for the timers' own grain.
  readonly property int grain: 100
  property var runs: []
  property int runCount: 0

  QtObject {
    id: fakeBar
    function run(command) {
      suite.runs = suite.runs.concat([command])
      suite.runCount = suite.runs.length
    }
  }

  // One of the check's own fakes inside a widget, found by a property only it has.
  function fake(widget, marker) {
    var parts = []
    for (var i = 0; i < widget.resources.length; i++) parts.push(widget.resources[i])
    for (var j = 0; j < widget.children.length; j++) parts.push(widget.children[j])
    for (var k = 0; k < parts.length; k++) if (marker in parts[k]) return parts[k]
    return null
  }
  function probe(widget) { return fake(widget, "starts") }
  function button(widget) { return fake(widget, "tooltipText") }

  function load() {
    var component = Qt.createComponent(Qt.resolvedUrl("../../../plugin/BarWidget.qml"))
    verify(component.status === Component.Ready, component.errorString())
    var widget = component.createObject(suite, { bar: fakeBar })
    verify(widget !== null)
    Desktop.copies = Desktop.copies.concat([widget])
    return widget
  }

  // One copy of the widget on a machine whose probe answers `answer`, with the
  // short intervals, once its load-time probe has finished.
  function make(answer) {
    Desktop.answer = answer
    var widget = load()
    verify("pollInterval" in widget, "the widget has no pollInterval")
    verify("followUpWindow" in widget, "the widget has no followUpWindow")
    verify("following" in widget, "the widget has no following")
    widget.pollInterval = suite.poll
    widget.followUpWindow = suite.window
    tryCompare(widget, "known", true)
    return widget
  }

  // Presses the button and waits for the command the click runs.
  function click(widget, clicks) {
    button(widget).pressed()
    tryCompare(suite, "runCount", clicks)
  }

  // No probe starts over eight poll intervals.
  function quiet(widget) {
    var before = probe(widget).starts
    wait(suite.poll * 8)
    compare(probe(widget).starts, before, "the widget is still probing")
  }

  // Waits for a follow-up to end by its bound, and holds it to the window's
  // length, measured from `since`, a moment before the click that opened it.
  function ranToTheBound(widget, since) {
    tryCompare(widget, "following", false, suite.window * 6)
    var lasted = Date.now() - since
    verify(lasted >= suite.window - suite.grain, "followed for " + lasted + " ms of " + suite.window)
  }

  function init() {
    runs = []
    runCount = 0
    Desktop.copies = []
  }

  function cleanup() {
    Desktop.copies.forEach(function (copy) { copy.destroy() })
    Desktop.copies = []
  }

  function test_ships_five_seconds_and_ten_minutes() {
    Desktop.answer = 0
    var widget = load()
    compare(widget.pollInterval, 5000)
    compare(widget.followUpWindow, 600000)
  }

  function test_current_never_polls() {
    var widget = make(0)
    compare(widget.following, false)
    quiet(widget)
    compare(probe(widget).starts, 1)
    compare(button(widget).active, false)
  }

  function test_update_ready_unclicked_never_polls() {
    var widget = make(12)
    compare(button(widget).active, true)
    compare(widget.following, false)
    quiet(widget)
    compare(probe(widget).starts, 1)
  }

  function test_launch_click_starts_no_follow_up() {
    var widget = make(0)
    click(widget, 1)
    verify(runs[0].endsWith(" launch"), runs[0])
    compare(widget.following, false)
    quiet(widget)
    compare(probe(widget).starts, 2)
  }

  function test_update_click_is_followed_until_current() {
    var widget = make(12)
    click(widget, 1)
    verify(runs[0].startsWith("omarchy-launch-floating-terminal-with-presentation "), runs[0])
    compare(widget.following, true)
    tryVerify(function () { return probe(widget).starts >= 5 })
    compare(widget.probeStatus, 12)
    compare(button(widget).active, true)
    Desktop.answer = 0
    tryCompare(widget, "probeStatus", 0)
    compare(button(widget).active, false)
    compare(widget.following, false)
    quiet(widget)
  }

  function test_declined_update_is_followed_only_to_the_bound() {
    var widget = make(12)
    var since = Date.now()
    click(widget, 1)
    ranToTheBound(widget, since)
    verify(probe(widget).starts >= 5, "probes: " + probe(widget).starts)
    compare(widget.probeStatus, 12)
    compare(button(widget).active, true)
    quiet(widget)
  }

  function test_a_second_click_restarts_the_bound() {
    var widget = make(12)
    click(widget, 1)
    wait(suite.window / 2)
    var since = Date.now()
    click(widget, 2)
    ranToTheBound(widget, since)
    quiet(widget)
  }

  function test_absent_polls_past_the_bound() {
    var widget = make(10)
    click(widget, 1)
    compare(widget.following, true)
    tryCompare(widget, "following", false, suite.window * 6)
    var before = probe(widget).starts
    tryVerify(function () { return probe(widget).starts >= before + 3 })
  }

  function test_failed_probe_polls_only_after_a_click_and_only_to_the_bound() {
    var widget = make(1)
    compare(button(widget).dimmed, true)
    compare(widget.following, false)
    quiet(widget)
    compare(probe(widget).starts, 1)
    var since = Date.now()
    click(widget, 1)
    tryVerify(function () { return probe(widget).starts >= 5 })
    ranToTheBound(widget, since)
    compare(widget.probeStatus, 1)
    quiet(widget)
  }

  // An update click, and one probe that fails while it is under way: the
  // follow-up has to outlast the failure, or the icon stays dimmed.
  function test_a_failed_probe_does_not_end_the_follow_up() {
    var widget = make(12)
    click(widget, 1)
    Desktop.answer = 1
    tryCompare(widget, "probeStatus", 1)
    var failed = probe(widget).starts
    tryVerify(function () { return probe(widget).starts >= failed + 3 })
    Desktop.answer = 0
    tryCompare(widget, "probeStatus", 0)
    compare(button(widget).active, false)
    compare(button(widget).dimmed, false)
    quiet(widget)
  }

  // A click on a failed probe, whose terminal decides again and updates: the
  // answer goes from the failure to 12 and only then to 0, and the follow-up
  // has to outlast the 12, or the icon stays urgent after the update.
  function test_a_click_on_a_failed_probe_is_followed_through_an_update() {
    var widget = make(1)
    click(widget, 1)
    Desktop.answer = 12
    tryCompare(widget, "probeStatus", 12)
    var updating = probe(widget).starts
    tryVerify(function () { return probe(widget).starts >= updating + 3 })
    Desktop.answer = 0
    tryCompare(widget, "probeStatus", 0)
    compare(button(widget).active, false)
    quiet(widget)
  }

  // An install click whose terminal decides again and finds an older package:
  // it updates, so the answer goes from 10 to 12 and only then to 0. The
  // follow-up has to outlast the 12, which absence polling does not.
  function test_an_install_click_is_followed_through_an_update() {
    var widget = make(10)
    click(widget, 1)
    Desktop.answer = 12
    tryCompare(widget, "probeStatus", 12)
    var updating = probe(widget).starts
    tryVerify(function () { return probe(widget).starts >= updating + 3 })
    Desktop.answer = 0
    tryCompare(widget, "probeStatus", 0)
    compare(button(widget).active, false)
    quiet(widget)
  }

  function test_every_copy_follows_the_click() {
    var clicked = make(12)
    var other = make(12)
    click(clicked, 1)
    compare(other.following, true)
    tryVerify(function () { return probe(other).starts >= 4 })
    Desktop.answer = 0
    tryCompare(other, "probeStatus", 0)
    tryCompare(clicked, "probeStatus", 0)
    compare(button(other).active, false)
    quiet(other)
    quiet(clicked)
  }

  function test_a_current_copy_is_not_set_polling() {
    var current = make(0)
    var stale = make(12)
    click(stale, 1)
    compare(current.following, false)
    tryVerify(function () { return probe(stale).starts >= 4 })
    compare(probe(current).starts, 1)
  }
}
