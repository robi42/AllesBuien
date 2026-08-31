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

            // Qualitative on purpose: Buienradar documents no colour-to-rate
            // mapping for these sprites, so numeric ticks would assert a
            // precision no source backs. Swatches and captions both come from
            // Rust, which is what keeps this file free of hardcoded colours.
            Item {
                anchors.left: parent.left
                anchors.bottom: parent.bottom
                anchors.margins: 16
                implicitWidth: legendColumn.implicitWidth
                implicitHeight: legendColumn.implicitHeight
                width: implicitWidth
                height: implicitHeight
                visible: radarController.frameCount > 0

                Rectangle {
                    anchors.fill: parent
                    anchors.margins: -8
                    radius: 6
                    color: themeController.themeAvailable ? themeController.themeLighterBackground : radarSystemPalette.alternateBase
                    opacity: 0.88
                    z: -1
                }

                ColumnLayout {
                    id: legendColumn
                    width: legendColumn.implicitWidth
                    height: legendColumn.implicitHeight
                    spacing: 4

                    Row {
                        spacing: 0
                        Repeater {
                            model: radarController.legendSwatches

                            Rectangle {
                                width: 15
                                height: 8
                                color: modelData
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 0
                        Repeater {
                            model: radarController.legendBuckets

                            Label {
                                text: modelData
                                font.pointSize: 8
                                font.bold: true
                                color: themeController.themeAvailable ? themeController.themeForeground : radarSystemPalette.text
                                opacity: 0.85
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                    }
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

                            // Where measurement stops and prediction starts. The
                            // window is 2 observations against 24 predictions, so
                            // this tick sits hard against the left edge on purpose:
                            // almost everything on screen is a forecast. Rust hides
                            // it for an all-observed or all-forecast run, where the
                            // boundary falls outside the track entirely.
                            Rectangle {
                                x: radarController.observedFraction * parent.width - width / 2
                                y: (parent.height - height) / 2
                                width: 2
                                height: 16
                                radius: 1
                                color: themeController.themeAvailable ? themeController.themeForeground : radarSystemPalette.text
                                visible: radarController.forecastBoundaryVisible
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

                    ColumnLayout {
                        spacing: 0
                        Layout.minimumWidth: 84

                        Label {
                            text: radarController.currentFrameLabel
                            color: themeController.themeAvailable ? themeController.themeForeground : radarSystemPalette.text
                            font.pointSize: 16
                            font.bold: true
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignRight
                        }

                        // Most of the window is prediction, and nothing else on
                        // screen says so. Full opacity marks the forecast case
                        // because that is the one worth noticing; accent would
                        // read at 3.7:1 here, which is too low for 9pt.
                        Label {
                            text: radarController.currentFrameObserved ? "OBSERVED" : "FORECAST"
                            visible: radarController.frameCount > 0
                            color: themeController.themeAvailable ? themeController.themeForeground : radarSystemPalette.text
                            opacity: radarController.currentFrameObserved ? 0.7 : 1.0
                            font.pointSize: 9
                            font.bold: true
                            font.letterSpacing: 0.8
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignRight
                        }
                    }
                }
            }
    }

    component ConditionIcon: Item {
        id: conditionIcon
        property alias source: iconImage.source
        property int slotSize: 34

        // The slot is reserved unconditionally. Sizing it from the image status
        // collapsed it to zero until the icon resolved, so every row it sat in
        // snapped sideways the moment the icons arrived. An icon that never
        // loads now leaves the reserved space empty instead of moving the text.
        implicitWidth: conditionIcon.slotSize
        implicitHeight: conditionIcon.slotSize

        Image {
            id: iconImage
            anchors.fill: parent
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            // Hidden unless decoded, so a failed load is blank, never a broken-image box.
            visible: iconImage.status === Image.Ready
        }
    }

    // One labelled measurement. Collapses itself when the station omitted the
    // value, so an absent reading leaves no stray label behind.
    component MetricChip: ColumnLayout {
        id: metricChip
        property string label: ""
        property string value: ""

        SystemPalette { id: metricPalette }

        spacing: 1
        visible: metricChip.value.length > 0

        Label {
            text: metricChip.label
            font.pointSize: 9
            font.bold: true
            font.letterSpacing: 0.8
            color: themeController.themeAvailable ? themeController.themeForeground : metricPalette.text
            opacity: 0.7
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Label {
            text: metricChip.value
            font.pointSize: 12
            font.bold: true
            color: themeController.themeAvailable ? themeController.themeForeground : metricPalette.text
            Layout.fillWidth: true
            elide: Text.ElideRight
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

            ColumnLayout {
                spacing: 0
                Layout.fillWidth: true

                Label {
                    text: weatherController.temperature
                    font.pointSize: 38
                    font.bold: true
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                }

                // Kept for the hero only, and kept to one line: at icon size the
                // 22 Buienradar codes are genuinely ambiguous, so the words earn
                // their space. It sits under the reading rather than beside it
                // because the longest of them ("Partly cloudy with light rain")
                // has no room left on that row without wrapping or eliding.
                Label {
                    text: weatherController.condition
                    font.pointSize: 13
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                    Layout.fillWidth: true
                    maximumLineCount: 1
                    elide: Text.ElideRight
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: metricsRow.implicitHeight + 20
            color: themeController.themeAvailable ? themeController.themeBackground : systemPalette.base
            radius: 8

            RowLayout {
                id: metricsRow
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                spacing: 12

                MetricChip {
                    label: "WIND"
                    value: weatherController.wind
                    Layout.fillWidth: true
                }
                MetricChip {
                    label: "FEELS"
                    value: weatherController.feelsLike
                    Layout.fillWidth: true
                }
                MetricChip {
                    label: "HUMID"
                    value: weatherController.humidity
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

                // The condition sentence used to live here and was the widest
                // column, wrapping most rows onto two lines. Icon, chance, and
                // high/low carry the same meaning in one line; this spacer takes
                // the width it freed and anchors both numeric columns right.
                Item {
                    Layout.fillWidth: true
                }

                RowLayout {
                    Layout.preferredWidth: 52
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
                    text: model.minTemperature + "° / " + model.maxTemperature + "°"
                    Layout.preferredWidth: 74
                    font.pointSize: 12
                    font.bold: true
                    color: themeController.themeAvailable ? themeController.themeForeground : systemPalette.text
                    horizontalAlignment: Text.AlignRight
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
