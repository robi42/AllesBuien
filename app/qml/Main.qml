import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// qmllint disable unqualified
ApplicationWindow {
    objectName: "rootWindow"
    width: 980
    height: 760
    visible: true
    title: "AllesBuien"
    color: themeController.themeAvailable ? themeController.themeBackground : systemPalette.window

    SystemPalette { id: systemPalette }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 16

        RadarView {
            objectName: "radarView"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 400
        }

        ColumnLayout {
            Layout.fillHeight: true
            Layout.preferredWidth: 320
            spacing: 16

            Item {
                objectName: "conditionsView"
                Layout.fillWidth: true
                Layout.preferredHeight: 200
            }

            Item {
                objectName: "forecastView"
                Layout.fillWidth: true
                Layout.fillHeight: true
            }

            Label {
                objectName: "attribution"
                text: "Bron: Buienradar.nl"
                font.pointSize: 10
                color: themeController.themeAvailable ? themeController.themeMuted : systemPalette.text
                Layout.alignment: Qt.AlignRight | Qt.AlignBottom
            }
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
        interval: themeController.pollIntervalMs
        repeat: true
        running: true
        onTriggered: themeController.poll()
    }

    Timer {
        interval: 50
        running: Qt.platform.pluginName === "offscreen"
        onTriggered: Qt.quit()
    }
}
// qmllint enable unqualified
