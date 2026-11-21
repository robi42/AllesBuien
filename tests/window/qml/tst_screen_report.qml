// The check of app/qml/ScreenReport.qml, the window's report of the screen it
// is on, which caps the zoom that follows Omarchy's text size and fits the
// size the window opens at. It runs on a desktop of two offscreen monitors of
// different sizes, two-screens.json beside this file, which only a child
// process can be given: the fourth test in tests/widget/tests/scripts.rs runs
// it inside `cargo test --workspace`. To run it alone, from the repository
// root:
//
//   QT_QPA_PLATFORM=offscreen:configfile=tests/window/qml/two-screens.json \
//     /usr/lib/qt6/bin/qmltestrunner -input tests/window/qml/tst_screen_report.qml
//
// It proves two things the whole window's GUI test cannot, since the gate's
// offscreen desktop there has one screen. A report arrives while the window
// is being created, before it is shown, so the opening size is fitted before
// the window appears. And when the window moves to the other monitor, every
// report carries that monitor's size, never one monitor's width beside the
// other's height. Every expectation comes from `Qt.application.screens`, so
// the host's display scale does not matter.
import QtQuick
import QtTest
import "../../../app/qml"

TestCase {
  id: suite
  name: "ScreenReport"

  Component {
    id: reportingWindow

    Window {
      id: window

      // Every report, with whether the window was shown when it arrived.
      property var reports: []

      width: 200
      height: 100
      visible: true

      ScreenReport {
        onReported: (screenWidth, screenHeight, workWidth, workHeight) => {
          window.reports.push({
            size: [screenWidth, screenHeight, workWidth, workHeight],
            shown: window.visible
          });
        }
      }
    }
  }

  // What a report of `screen` carries.
  function whole(screen) {
    return [screen.width, screen.height, screen.desktopAvailableWidth, screen.desktopAvailableHeight];
  }

  function test_a_report_arrives_before_the_window_is_shown() {
    const window = createTemporaryObject(reportingWindow, suite);
    verify(window !== null, "the window was not created");
    verify(window.reports.length > 0, "no report arrived while the window was created");
    const first = window.reports[0];
    verify(!first.shown, "the first report arrived after the window was shown");
    compare(first.size, suite.whole(Qt.application.screens[0]), "the first report is not the primary screen");
  }

  function test_every_report_of_a_move_carries_one_screen() {
    const screens = Qt.application.screens;
    compare(screens.length, 2, "the desktop has two screens");
    verify(screens[0].width !== screens[1].width && screens[0].height !== screens[1].height,
           "the two screens differ in both width and height, or a mix of them could not be seen");
    const window = createTemporaryObject(reportingWindow, suite);
    tryVerify(() => window.visible, 5000, "the window is shown");
    const before = window.reports.length;

    window.screen = screens[1];

    tryVerify(() => window.reports.length > before, 5000, "the move is reported");
    for (const report of window.reports.slice(before)) {
      compare(report.size, suite.whole(screens[1]), "a report of the move mixed two screens");
    }
  }
}
