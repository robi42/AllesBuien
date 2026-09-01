// Delegates below reach the component root's id for its injected `theme`.
// Bound component behaviour is what makes that access resolvable instead of
// a late context lookup; every model role in this file is a required property.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: radarPanel

    required property var radarController
    required property Theme theme

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
                source: radarPanel.radarController.currentFrameUrl
                fillMode: Image.PreserveAspectCrop
                visible: radarPanel.radarController.frameCount > 0
            }
        }

        // Qualitative on purpose: Buienradar documents no colour-to-rate
        // mapping for these sprites, so numeric ticks would assert a
        // precision no source backs. Swatches and captions both come from
        // Rust, which is what keeps this file free of hardcoded colours.
        Item {
            id: legend
            anchors.left: parent.left
            anchors.bottom: parent.bottom
            anchors.margins: 16
            implicitWidth: legendColumn.implicitWidth
            implicitHeight: legendColumn.implicitHeight
            width: legend.implicitWidth
            height: legend.implicitHeight
            visible: radarPanel.radarController.frameCount > 0

            Rectangle {
                anchors.fill: parent
                anchors.margins: -8
                radius: 6
                color: radarPanel.theme.surface
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
                        model: radarPanel.radarController.legendSwatches

                        Rectangle {
                            id: swatch
                            required property var modelData

                            width: 15
                            height: 8
                            color: swatch.modelData
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    Repeater {
                        model: radarPanel.radarController.legendBuckets

                        Label {
                            id: bucketLabel
                            required property var modelData

                            text: bucketLabel.modelData
                            font.pointSize: 8
                            font.bold: true
                            color: radarPanel.theme.text
                            opacity: 0.85
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                }
            }
        }

        Label {
            id: radarError
            objectName: "radarError"
            text: radarPanel.radarController.status
            visible: radarError.text.length > 0
            color: radarPanel.theme.error
            anchors.centerIn: parent
            width: Math.min(radarError.implicitWidth, parent.width - 48)
            wrapMode: Text.Wrap
            maximumLineCount: 3
            elide: Text.ElideRight
            horizontalAlignment: Text.AlignHCenter
            font.pointSize: 14
            font.bold: true
            Rectangle {
                anchors.fill: parent
                anchors.margins: -8
                color: radarPanel.theme.background
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
        color: radarPanel.theme.surface
        radius: 12

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 16
            anchors.rightMargin: 20
            spacing: 20

            Button {
                id: playButton
                text: radarPanel.radarController.playing ? "Pause" : "Play"
                enabled: radarPanel.radarController.frameCount > 1
                Layout.preferredHeight: 36
                Layout.preferredWidth: 80

                background: Rectangle {
                    color: playButton.down ? radarPanel.theme.selection : radarPanel.theme.buttonFill
                    radius: 6
                }
                contentItem: Text {
                    text: playButton.text
                    color: radarPanel.theme.buttonText
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    font.bold: true
                    font.pointSize: 11
                }

                onClicked: radarPanel.radarController.playing ? radarPanel.radarController.pause() : radarPanel.radarController.play()
            }

            Slider {
                id: frameSlider
                Layout.fillWidth: true
                from: 0
                to: Math.max(0, radarPanel.radarController.frameCount - 1)
                value: radarPanel.radarController.currentFrameIndex
                stepSize: 1
                enabled: radarPanel.radarController.frameCount > 1

                background: Rectangle {
                    id: sliderGroove
                    x: frameSlider.leftPadding
                    y: frameSlider.topPadding + frameSlider.availableHeight / 2 - sliderGroove.height / 2
                    implicitWidth: 200
                    implicitHeight: 6
                    width: frameSlider.availableWidth
                    height: sliderGroove.implicitHeight
                    radius: 3
                    color: radarPanel.theme.sliderTrack

                    Rectangle {
                        width: frameSlider.visualPosition * parent.width
                        height: parent.height
                        color: radarPanel.theme.accent
                        radius: 3
                    }

                    // Where measurement stops and prediction starts. The
                    // window is 2 observations against 24 predictions, so
                    // this tick sits hard against the left edge on purpose:
                    // almost everything on screen is a forecast. Rust hides
                    // it for an all-observed or all-forecast run, where the
                    // boundary falls outside the track entirely.
                    Rectangle {
                        id: forecastBoundary
                        x: radarPanel.radarController.observedFraction * parent.width - forecastBoundary.width / 2
                        y: (parent.height - forecastBoundary.height) / 2
                        width: 2
                        height: 16
                        radius: 1
                        color: radarPanel.theme.text
                        visible: radarPanel.radarController.forecastBoundaryVisible
                    }
                }
                handle: Rectangle {
                    id: sliderHandle
                    x: frameSlider.leftPadding + frameSlider.visualPosition * (frameSlider.availableWidth - sliderHandle.width)
                    y: frameSlider.topPadding + frameSlider.availableHeight / 2 - sliderHandle.height / 2
                    implicitWidth: 18
                    implicitHeight: 18
                    radius: 9
                    color: radarPanel.theme.accent
                }

                onMoved: radarPanel.radarController.seek(Math.round(frameSlider.value))
            }

            ColumnLayout {
                spacing: 0
                Layout.minimumWidth: 84

                Label {
                    text: radarPanel.radarController.currentFrameLabel
                    color: radarPanel.theme.text
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
                    text: radarPanel.radarController.currentFrameObserved ? "OBSERVED" : "FORECAST"
                    visible: radarPanel.radarController.frameCount > 0
                    color: radarPanel.theme.text
                    opacity: radarPanel.radarController.currentFrameObserved ? 0.7 : 1.0
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
