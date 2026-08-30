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

    component RadarPanel: Item {
        SystemPalette { id: radarSystemPalette }

        ColumnLayout {
            anchors.fill: parent
            spacing: 8

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Image {
                    anchors.centerIn: parent
                    width: Math.min(parent.width, parent.height * (1058 / 915))
                    height: width * (915 / 1058)
                    source: "../assets/basemap-nl.png"
                    fillMode: Image.Stretch

                    Image {
                        anchors.fill: parent
                        source: radarController.currentFrameUrl
                        fillMode: Image.Stretch
                        visible: radarController.frameCount > 0
                    }
                }
            }

            Label {
                objectName: "radarError"
                text: radarController.status
                visible: text.length > 0
                color: themeController.themeAvailable ? themeController.themeAccent : "red"
                Layout.alignment: Qt.AlignHCenter
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.margins: 8
                spacing: 12

                Button {
                    text: radarController.playing ? "Pause" : "Play"
                    enabled: radarController.frameCount > 1
                    palette.button: themeController.themeAvailable ? themeController.themeLighterBackground : radarSystemPalette.button
                    palette.buttonText: themeController.themeAvailable ? themeController.themeForeground : radarSystemPalette.buttonText
                    onClicked: radarController.playing ? radarController.pause() : radarController.play()
                }

                Slider {
                    Layout.fillWidth: true
                    from: 0
                    to: Math.max(0, radarController.frameCount - 1)
                    value: radarController.currentFrameIndex
                    stepSize: 1
                    enabled: radarController.frameCount > 1

                    onMoved: {
                        if (radarController.playing) {
                            radarController.pause()
                        }
                        radarController.currentFrameIndex = Math.round(value)
                        radarController.currentFrameUrl = "image://radar/" + radarController.generation + "/" + Math.round(value)
                        if (radarController.timestamps && Math.round(value) < radarController.timestamps.length) {
                            radarController.currentFrameLabel = radarController.timestamps[Math.round(value)]
                        }
                    }
                }

                Label {
                    text: radarController.currentFrameLabel
                    color: themeController.themeAvailable ? themeController.themeForeground : radarSystemPalette.text
                    Layout.minimumWidth: 120
                    horizontalAlignment: Text.AlignRight
                }
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 16

        RadarPanel {
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
