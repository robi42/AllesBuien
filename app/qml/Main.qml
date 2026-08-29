import QtQuick
import QtQuick.Controls

// Placeholder only; todo 17 replaces this root window.
ApplicationWindow {
    objectName: "rootWindow"
    width: 640
    height: 480
    visible: true
    title: "AllesBuien"

    Timer {
        interval: 50
        running: Qt.platform.pluginName === "offscreen"
        onTriggered: Qt.quit()
    }
}
