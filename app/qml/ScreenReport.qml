import QtQuick

// Reports the screen its window is on, whole: the screen's size and the
// available area Qt reports, which on a desktop of several monitors spans them
// all. The theme bridge caps the window's zoom by it and fits the opening size
// to it, so a report has to describe one screen, never parts of two.
//
// The four properties below mirror Qt's attached `Screen` values. They exist
// to raise a change signal whenever one of those changes, and for a test to
// read, and the report never sends them. When a window moves between monitors,
// Qt has already switched every `Screen` value by the time it emits the first
// change, but the mirrors catch up one handler at a time: moving from
// 2560×1440 to 1280×800, the width's handler runs while the height mirror
// still reads 1440. So `report` reads `Screen` itself, and four handlers that
// fire for one move send the same screen four times, which the bridge takes
// as one.
//
// No Qt version promises that the mirrors change while the window is being
// created, so the completion call is the guarantee that the bridge hears of a
// screen at all. The file stands alone so that a test can move it between two
// monitors, which the whole window cannot be without its three controllers:
// tests/window/qml/tst_screen_report.qml.
Item {
    id: screenReport

    signal reported(int screenWidth, int screenHeight, int workWidth, int workHeight)

    readonly property int screenWidth: Screen.width
    readonly property int screenHeight: Screen.height
    readonly property int workWidth: Screen.desktopAvailableWidth
    readonly property int workHeight: Screen.desktopAvailableHeight

    function report() {
        screenReport.reported(Screen.width, Screen.height, Screen.desktopAvailableWidth, Screen.desktopAvailableHeight);
    }

    onScreenWidthChanged: screenReport.report()
    onScreenHeightChanged: screenReport.report()
    onWorkWidthChanged: screenReport.report()
    onWorkHeightChanged: screenReport.report()
    Component.onCompleted: screenReport.report()
}
