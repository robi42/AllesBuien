// qmllint disable unqualified
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    SystemPalette { id: systemPalette }

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            Image {
                id: basemap
                anchors.centerIn: parent
                width: Math.min(parent.width, parent.height * (1058 / 915))
                height: width * (915 / 1058)
                source: "../assets/basemap-nl.png"
                fillMode: Image.Stretch

                Image {
                    id: overlay
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
            spacing: 12
            Layout.margins: 8

            Button {
                text: radarController.playing ? "Pause" : "Play"
                enabled: radarController.frameCount > 1
                onClicked: radarController.playing ? radarController.pause() : radarController.play()
                
                palette.button: themeController.themeAvailable ? themeController.themeLighterBackground : systemPalette.button
                palette.buttonText: themeController.themeAvailable ? themeController.themeForeground : systemPalette.buttonText
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
                color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                Layout.minimumWidth: 120
                horizontalAlignment: Text.AlignRight
            }
        }
    }
}
