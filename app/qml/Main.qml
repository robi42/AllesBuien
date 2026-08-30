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

    component ConditionsView: Item {
        SystemPalette { id: systemPalette }

        ColumnLayout {
            anchors.fill: parent
            spacing: 8

            Label {
                text: weatherController.stationName
                font.pointSize: 14
                font.bold: true
                color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }

            Label {
                text: weatherController.temperature
                font.pointSize: 24
                font.bold: true
                color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                Layout.fillWidth: true
            }

            Label {
                text: weatherController.condition
                font.pointSize: 12
                color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }

            Label {
                text: weatherController.wind
                font.pointSize: 12
                color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                Layout.fillWidth: true
            }

            Label {
                text: weatherController.rainSummary
                font.pointSize: 12
                color: themeController.themeAvailable ? themeController.themeAccent : systemPalette.highlight
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                visible: text.length > 0
            }

            Label {
                objectName: "conditionsError"
                text: weatherController.status
                visible: text.length > 0
                color: themeController.themeAvailable ? themeController.themeAccent : "red"
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }
        }
    }

    component ForecastView: ListView {
        id: forecastList
        clip: true
        model: weatherController

        SystemPalette { id: systemPalette }

        delegate: Item {
            width: ListView.view.width
            height: 40

            RowLayout {
                anchors.fill: parent
                anchors.margins: 4
                spacing: 8

                Label {
                    text: model.day
                    Layout.preferredWidth: 60
                    font.bold: true
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                }

                Label {
                    text: model.minTemperature + "° / " + model.maxTemperature + "°"
                    Layout.preferredWidth: 80
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                }

                Label {
                    text: model.rainChance > 0 ? (model.rainChance + "%") : ""
                    Layout.preferredWidth: 40
                    color: themeController.themeAvailable ? themeController.themeAccent : systemPalette.highlight
                }

                Label {
                    text: model.condition
                    Layout.fillWidth: true
                    color: themeController.themeAvailable ? themeController.themeMuted : systemPalette.text
                    elide: Text.ElideRight
                }
            }

            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 1
                color: themeController.themeAvailable ? themeController.themeSelection : systemPalette.alternateBase
                opacity: 0.5
            }
        }

        Label {
            objectName: "forecastError"
            anchors.centerIn: parent
            text: weatherController.status
            visible: forecastList.count === 0 && text.length > 0
            color: themeController.themeAvailable ? themeController.themeAccent : "red"
            width: parent.width - 16
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
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

            ConditionsView {
                objectName: "conditionsView"
                Layout.fillWidth: true
                Layout.preferredHeight: 200
            }

            ForecastView {
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
