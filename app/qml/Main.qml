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

        Item {
            id: mapContainer
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: controlsContainer.top
            anchors.bottomMargin: 12

            Image {
                anchors.fill: parent
                source: "../assets/basemap-nl.png"
                fillMode: Image.PreserveAspectCrop

                Image {
                    objectName: "radarOverlay"
                    anchors.fill: parent
                    source: radarController.currentFrameUrl
                    fillMode: Image.PreserveAspectCrop
                    visible: radarController.frameCount > 0
                }
            }

            Label {
                objectName: "radarError"
                text: radarController.status
                visible: text.length > 0
                color: themeController.themeAvailable ? themeController.themeAccent : "red"
                anchors.centerIn: parent
                width: Math.min(implicitWidth, parent.width - 48)
                wrapMode: Text.Wrap
                maximumLineCount: 3
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignHCenter
                font.pointSize: 14
                font.bold: true
                Rectangle {
                    anchors.fill: parent
                    anchors.margins: -8
                    color: themeController.themeAvailable ? themeController.themeBackground : systemPalette.window
                    opacity: 0.8
                    radius: 4
                    z: -1
                }
            }
        }

        Rectangle {
            id: controlsContainer
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            height: 56
            color: themeController.themeAvailable ? themeController.themeLighterBackground : radarSystemPalette.alternateBase
            radius: 12

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 20
                    spacing: 20

                    Button {
                        id: playButton
                        text: radarController.playing ? "Pause" : "Play"
                        enabled: radarController.frameCount > 1
                        Layout.preferredHeight: 36
                        Layout.preferredWidth: 80
                        
                        background: Rectangle {
                            color: playButton.down ? (themeController.themeAvailable ? themeController.themeSelection : radarSystemPalette.mid) 
                                                   : (themeController.themeAvailable ? themeController.themeMuted : radarSystemPalette.button)
                            radius: 6
                        }
                        contentItem: Text {
                            text: playButton.text
                            color: themeController.themeAvailable ? themeController.themeBackground : radarSystemPalette.buttonText
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                            font.bold: true
                            font.pointSize: 11
                        }

                        onClicked: radarController.playing ? radarController.pause() : radarController.play()
                    }

                    Slider {
                        id: frameSlider
                        Layout.fillWidth: true
                        from: 0
                        to: Math.max(0, radarController.frameCount - 1)
                        value: radarController.currentFrameIndex
                        stepSize: 1
                        enabled: radarController.frameCount > 1

                        background: Rectangle {
                            x: frameSlider.leftPadding
                            y: frameSlider.topPadding + frameSlider.availableHeight / 2 - height / 2
                            implicitWidth: 200
                            implicitHeight: 6
                            width: frameSlider.availableWidth
                            height: implicitHeight
                            radius: 3
                            color: themeController.themeAvailable ? themeController.themeBackground : radarSystemPalette.midlight

                            Rectangle {
                                width: frameSlider.visualPosition * parent.width
                                height: parent.height
                                color: themeController.themeAvailable ? themeController.themeAccent : radarSystemPalette.highlight
                                radius: 3
                            }
                        }
                        handle: Rectangle {
                            x: frameSlider.leftPadding + frameSlider.visualPosition * (frameSlider.availableWidth - width)
                            y: frameSlider.topPadding + frameSlider.availableHeight / 2 - height / 2
                            implicitWidth: 18
                            implicitHeight: 18
                            radius: 9
                            color: themeController.themeAvailable ? themeController.themeAccent : radarSystemPalette.highlight
                        }

                        onMoved: radarController.seek(Math.round(value))
                    }

                    Label {
                        text: radarController.currentFrameLabel
                        color: themeController.themeAvailable ? themeController.themeForeground : radarSystemPalette.text
                        font.pointSize: 16
                        font.bold: true
                        Layout.minimumWidth: 60
                        horizontalAlignment: Text.AlignRight
                    }
                }
            }
    }

    component ConditionIcon: Item {
        id: conditionIcon
        property alias source: iconImage.source
        property int slotSize: 34

        implicitWidth: iconImage.status === Image.Ready ? conditionIcon.slotSize : 0
        implicitHeight: conditionIcon.slotSize

        Image {
            id: iconImage
            anchors.fill: parent
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            visible: status === Image.Ready
        }
    }

    component ConditionsView: ColumnLayout {
        SystemPalette { id: systemPalette }
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Label {
                text: weatherController.stationName
                font.pointSize: 14
                font.bold: true
                font.letterSpacing: 0.5
                color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                Layout.fillWidth: true
                elide: Text.ElideRight
            }

            Label {
                text: weatherController.observedLabel
                visible: text.length > 0
                font.pointSize: 10
                font.bold: true
                color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                opacity: 0.7
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            ConditionIcon {
                source: weatherController.conditionIconUrl
                slotSize: 56
                Layout.preferredWidth: implicitWidth
                Layout.preferredHeight: implicitHeight
            }

            Label {
                text: weatherController.temperature
                font.pointSize: 38
                font.bold: true
                color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
            }

            Label {
                text: weatherController.condition
                font.pointSize: 13
                color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: windRow.implicitHeight + 20
            color: themeController.themeAvailable ? themeController.themeBackground : systemPalette.base
            radius: 8

            RowLayout {
                id: windRow
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.margins: 10
                spacing: 12

                Label {
                    text: "Wind"
                    font.pointSize: 11
                    font.bold: true
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                    opacity: 0.7
                    Layout.preferredWidth: 50
                }
                Label {
                    text: weatherController.wind
                    font.pointSize: 11
                    font.bold: true
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                    Layout.fillWidth: true
                }
            }
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

    component ForecastView: ListView {
        id: forecastList
        clip: true
        model: weatherController
        interactive: false

        SystemPalette { id: systemPalette }

        delegate: Item {
            width: ListView.view.width
            height: Math.max(44, rowLayout.implicitHeight + 12)

            RowLayout {
                id: rowLayout
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: 4
                anchors.rightMargin: 4
                spacing: 10

                Label {
                    text: model.weekday
                    Layout.preferredWidth: 40
                    font.pointSize: 12
                    font.bold: true
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                }

                ConditionIcon {
                    source: model.iconUrl
                    Layout.preferredWidth: implicitWidth
                    Layout.preferredHeight: implicitHeight
                }

                Label {
                    text: model.minTemperature + "° / " + model.maxTemperature + "°"
                    Layout.preferredWidth: 70
                    font.pointSize: 12
                    font.bold: true
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                    horizontalAlignment: Text.AlignRight
                }

                RowLayout {
                    Layout.preferredWidth: 46
                    spacing: 4
                    Rectangle {
                        Layout.preferredWidth: 8
                        Layout.preferredHeight: 8
                        radius: 4
                        color: themeController.themeAvailable ? themeController.themeAccent : systemPalette.highlight
                        visible: model.rainChance > 0
                    }
                    Label {
                        text: model.rainChance > 0 ? (model.rainChance + "%") : ""
                        font.pointSize: 11
                        font.bold: true
                        color: themeController.themeAvailable ? themeController.themeAccent : systemPalette.highlight
                    }
                }

                Label {
                    text: model.condition
                    Layout.fillWidth: true
                    font.pointSize: 11
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                    wrapMode: Text.Wrap
                }
            }

            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 1
                color: themeController.themeAvailable ? themeController.themeSelection : systemPalette.alternateBase
                opacity: 0.3
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

    component RainView: ColumnLayout {
        id: rainView
        spacing: 8

        SystemPalette { id: systemPalette }

        readonly property color textColor: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
        readonly property color barColor: themeController.themeAvailable ? themeController.themeAccent : systemPalette.highlight

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Label {
                text: "Rain expectation"
                font.pointSize: 11
                font.bold: true
                color: rainView.textColor
                opacity: 0.7
            }

            Label {
                text: weatherController.rainSummary
                font.pointSize: 11
                font.bold: true
                color: weatherController.rainWet ? rainView.barColor : rainView.textColor
                opacity: weatherController.rainWet ? 1.0 : 0.7
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignRight
                elide: Text.ElideRight
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 48
            color: themeController.themeAvailable ? themeController.themeBackground : systemPalette.base
            radius: 8

            Item {
                id: plot
                anchors.fill: parent
                anchors.margins: 8

                readonly property int cellCount: Math.max(1, weatherController.rainLevels.length)
                readonly property real cellWidth: plot.width / plot.cellCount

                Repeater {
                    model: weatherController.rainLevels

                    Rectangle {
                        x: index * plot.cellWidth
                        width: Math.max(2, plot.cellWidth - 4)
                        y: plot.height - height
                        height: Math.max(2, plot.height * modelData)
                        radius: 2
                        color: rainView.barColor
                        opacity: modelData > 0 ? 1.0 : 0.45
                    }
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 1
                    color: rainView.textColor
                    opacity: 0.4
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 16

            Item {
                id: axis
                anchors.fill: parent
                anchors.leftMargin: 8
                anchors.rightMargin: 8

                Repeater {
                    model: weatherController.rainAxisLabels

                    Label {
                        text: modelData
                        visible: text.length > 0
                        font.pointSize: 9
                        color: rainView.textColor
                        opacity: 0.7
                        x: Math.max(0, Math.min(axis.width - width, (index + 0.5) * plot.cellWidth - width / 2))
                    }
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 32

            RadarPanel {
                objectName: "radarView"
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 400
            }

            Rectangle {
                Layout.preferredWidth: 420
                Layout.fillHeight: true
                color: themeController.themeAvailable ? themeController.themeLighterBackground : systemPalette.alternateBase
                radius: 16

                ColumnLayout {
                    id: rightCol
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 16

                    ConditionsView {
                        objectName: "conditionsView"
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: themeController.themeAvailable ? themeController.themeSelection : systemPalette.mid
                        opacity: 0.5
                    }

                    ForecastView {
                        objectName: "forecastView"
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.preferredHeight: contentHeight
                    }

                    Label {
                        objectName: "attribution"
                        text: "Bron: Buienradar.nl"
                        font.pointSize: 10
                        font.bold: true
                        color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                        opacity: 0.7
                        Layout.alignment: Qt.AlignRight
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 132
            color: themeController.themeAvailable ? themeController.themeLighterBackground : systemPalette.alternateBase
            radius: 16

            RainView {
                anchors.fill: parent
                anchors.margins: 16
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
