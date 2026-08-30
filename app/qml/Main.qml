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
        Image {
            width: 320
            height: 277
            fillMode: Image.PreserveAspectFit
            source: radarController.currentFrameUrl
            visible: radarController.frameCount > 0
        }
        Label { text: radarController.currentFrameLabel }
        Label { text: radarController.status; visible: text.length > 0 }
        Button {
            text: radarController.playing ? "Pause" : "Play"
            enabled: radarController.frameCount > 1
            onClicked: radarController.playing ? radarController.pause() : radarController.play()
        }
    }

    Timer {
        interval: weatherController.refreshIntervalMs
        repeat: true
        running: true
        onTriggered: weatherController.refresh()
    }

    Timer {
        interval: radarController.refreshIntervalMs
        repeat: true
        running: true
        onTriggered: radarController.refresh()
    }

    Timer {
        interval: radarController.playbackIntervalMs
        repeat: true
        running: radarController.playing
        onTriggered: radarController.advanceFrame()
    }

    Timer {
        interval: 50
        running: Qt.platform.pluginName === "offscreen"
        onTriggered: Qt.quit()
    }
}
// qmllint enable unqualified
