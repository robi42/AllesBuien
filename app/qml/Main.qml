import QtQuick
import QtQuick.Controls

// Placeholder only; todo 17 replaces this root window.
// qmllint disable unqualified
ApplicationWindow {
    objectName: "rootWindow"
    width: 640
    height: 480
    visible: true
    title: "AllesBuien"

    Column {
        anchors.centerIn: parent
        spacing: 8

        Label { text: weatherController.stationName + " · " + weatherController.temperature }
        Label { text: weatherController.condition + " · wind " + weatherController.wind }
        Label { text: weatherController.rainSummary }
        Label { text: weatherController.status; visible: text.length > 0 }
    }

    Timer {
        interval: weatherController.refreshIntervalMs
        repeat: true
        running: true
        onTriggered: weatherController.refresh()
    }

    Timer {
        interval: 50
        running: Qt.platform.pluginName === "offscreen"
        onTriggered: Qt.quit()
    }
}
// qmllint enable unqualified
