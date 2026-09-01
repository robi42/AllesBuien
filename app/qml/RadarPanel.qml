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

    // Used on both axes of the handle; every other one-off size below is written
    // where it is used, and anything that lands on the 4 px scale comes from the
    // theme rather than being restated here.
    readonly property int handleSize: 18

    Item {
        id: mapContainer
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: controlsContainer.top
        anchors.bottomMargin: radarPanel.theme.spaceMd

        Image {
            id: basemap
            anchors.fill: parent
            source: "../assets/basemap-nl.png"
            fillMode: Image.PreserveAspectCrop
            // The marker below is positioned in the raster's own coordinates, and
            // PreserveAspectCrop scales the raster past the item on one axis, so a
            // location outside the visible crop has to be cut off with it.
            clip: true

            // PreserveAspectCrop paints the source scaled by max(w/sw, h/sh) and
            // centred, so paintedWidth/paintedHeight are the on-screen size of the
            // whole raster and these offsets are its top-left corner. Anchoring the
            // marker to them is what keeps it on its coastline as the window resizes.
            readonly property real rasterOriginX: (basemap.width - basemap.paintedWidth) / 2
            readonly property real rasterOriginY: (basemap.height - basemap.paintedHeight) / 2

            Image {
                objectName: "radarOverlay"
                anchors.fill: parent
                source: radarPanel.radarController.currentFrameUrl
                fillMode: Image.PreserveAspectCrop
                visible: radarPanel.radarController.frameCount > 0
            }

            // Rust projects the configured latitude/longitude onto the radar
            // raster and hands QML two fractions; everything here is geometry.
            Item {
                id: locationMarker
                width: radarPanel.theme.spaceLg
                height: radarPanel.theme.spaceLg
                x: basemap.rasterOriginX + radarPanel.radarController.markerXFraction * basemap.paintedWidth - locationMarker.width / 2
                y: basemap.rasterOriginY + radarPanel.radarController.markerYFraction * basemap.paintedHeight - locationMarker.height / 2
                visible: radarPanel.radarController.markerVisible && basemap.status === Image.Ready

                Rectangle {
                    anchors.fill: parent
                    radius: width / 2
                    color: radarPanel.theme.background
                    opacity: 0.55
                }

                Rectangle {
                    anchors.centerIn: parent
                    width: radarPanel.theme.spaceSm
                    height: radarPanel.theme.spaceSm
                    radius: width / 2
                    color: radarPanel.theme.accent
                    border.width: 2
                    border.color: radarPanel.theme.buttonText
                }
            }
        }

        // A padded card inset from the map's corner by the same step the panels
        // use, rather than a ramp floating loose on the terrain. It sits bottom
        // left because that corner is open sea and northern France — the one
        // part of this frame that never carries Dutch weather.
        //
        // Reproduces the scale Buienradar prints on its own map. Buckets are
        // uneven (2, 3, 5, 90, open-ended), so each gets an equal block rather
        // than a proportional gradient. Swatches, captions, and unit all come
        // from Rust, which keeps this file free of hardcoded colours.
        Rectangle {
            id: legend
            anchors.left: parent.left
            anchors.bottom: parent.bottom
            anchors.margins: radarPanel.theme.spaceMd
            implicitWidth: legendColumn.implicitWidth + radarPanel.theme.spaceMd * 2
            implicitHeight: legendColumn.implicitHeight + radarPanel.theme.spaceMd * 2
            radius: radarPanel.theme.radiusMd
            color: radarPanel.theme.surface
            visible: radarPanel.radarController.frameCount > 0

            ColumnLayout {
                id: legendColumn
                anchors.fill: parent
                anchors.margins: radarPanel.theme.spaceMd
                spacing: radarPanel.theme.spaceXs

                Label {
                    text: radarPanel.radarController.legendUnit
                    font.pointSize: radarPanel.theme.fontCaption
                    font.bold: true
                    font.letterSpacing: radarPanel.theme.capsTracking
                    color: radarPanel.theme.text
                    opacity: radarPanel.theme.emphasisMid
                }

                RowLayout {
                    spacing: radarPanel.theme.spaceXs
                    Repeater {
                        model: radarPanel.radarController.legendSwatches

                        ColumnLayout {
                            id: bucket
                            required property int index
                            required property var modelData

                            spacing: 2

                            Rectangle {
                                Layout.fillWidth: true
                                implicitHeight: radarPanel.theme.spaceSm
                                radius: radarPanel.theme.radiusSm
                                color: bucket.modelData
                            }

                            Label {
                                text: radarPanel.radarController.legendBuckets[bucket.index]
                                font.pointSize: radarPanel.theme.fontCaption
                                font.features: ({ tnum: 1 })
                                color: radarPanel.theme.text
                                opacity: radarPanel.theme.emphasisMid
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignHCenter
                            }
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
            width: Math.min(radarError.implicitWidth, parent.width - radarPanel.theme.spaceXl * 2)
            wrapMode: Text.Wrap
            maximumLineCount: 3
            elide: Text.ElideRight
            horizontalAlignment: Text.AlignHCenter
            font.pointSize: radarPanel.theme.fontBody
            font.bold: true
            Rectangle {
                anchors.fill: parent
                anchors.margins: -radarPanel.theme.spaceSm
                color: radarPanel.theme.background
                opacity: radarPanel.theme.scrimOpacity
                radius: radarPanel.theme.radiusSm
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
        radius: radarPanel.theme.radiusLg

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: radarPanel.theme.spaceLg
            anchors.rightMargin: radarPanel.theme.spaceXl
            spacing: radarPanel.theme.spaceXl

            Button {
                id: playButton
                text: radarPanel.radarController.playing ? "Pause" : "Play"
                enabled: radarPanel.radarController.frameCount > 1
                Layout.preferredHeight: 36
                Layout.preferredWidth: 80

                background: Rectangle {
                    color: playButton.down ? radarPanel.theme.selection : radarPanel.theme.buttonFill
                    radius: radarPanel.theme.radiusSm
                }
                contentItem: Text {
                    text: playButton.text
                    color: radarPanel.theme.buttonText
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    font.bold: true
                    font.pointSize: radarPanel.theme.fontValue
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
                    radius: sliderGroove.height / 2
                    color: radarPanel.theme.sliderTrack

                    // Where measurement stops and prediction starts. The window
                    // is 2 observations against 24 predictions, so a lone hairline
                    // this far left was easy to miss; the travelled part of the
                    // track changes colour at the same x instead, which makes the
                    // boundary an edge rather than something to hunt for.
                    // Clamping is what handles Rust's degenerate runs: an
                    // all-forecast run reports a fraction below 0 and comes out
                    // fully accent, an all-observed one reports above 1 and comes
                    // out fully muted, and neither draws the divider.
                    readonly property real boundaryX: Math.max(0, Math.min(1, radarPanel.radarController.observedFraction)) * sliderGroove.width
                    readonly property real travelledX: frameSlider.visualPosition * sliderGroove.width
                    readonly property real observedX: Math.min(sliderGroove.travelledX, sliderGroove.boundaryX)

                    Rectangle {
                        width: sliderGroove.observedX
                        height: sliderGroove.height
                        radius: sliderGroove.radius
                        color: radarPanel.theme.text
                        opacity: 0.55
                    }

                    Rectangle {
                        x: sliderGroove.observedX
                        width: Math.max(0, sliderGroove.travelledX - sliderGroove.observedX)
                        height: sliderGroove.height
                        radius: sliderGroove.radius
                        color: radarPanel.theme.accent
                    }

                    // Light edges over the dark untravelled track, dark core over
                    // the accent and the muted fill: the divider has to stay
                    // legible on all three, because the handle can be either side.
                    Rectangle {
                        id: forecastBoundary
                        x: sliderGroove.boundaryX - forecastBoundary.width / 2
                        y: (sliderGroove.height - forecastBoundary.height) / 2
                        width: 4
                        height: 18
                        radius: 1
                        color: radarPanel.theme.background
                        border.width: 1
                        border.color: radarPanel.theme.text
                        visible: radarPanel.radarController.forecastBoundaryVisible
                    }
                }
                handle: Rectangle {
                    id: sliderHandle
                    x: frameSlider.leftPadding + frameSlider.visualPosition * (frameSlider.availableWidth - sliderHandle.width)
                    y: frameSlider.topPadding + frameSlider.availableHeight / 2 - sliderHandle.height / 2
                    implicitWidth: radarPanel.handleSize
                    implicitHeight: radarPanel.handleSize
                    radius: sliderHandle.width / 2
                    color: radarPanel.theme.accent
                    border.width: 2
                    border.color: radarPanel.theme.surface
                }

                onMoved: radarPanel.radarController.seek(Math.round(frameSlider.value))
            }

            ColumnLayout {
                spacing: 0
                Layout.minimumWidth: 84

                Label {
                    text: radarPanel.radarController.currentFrameLabel
                    color: radarPanel.theme.text
                    font.pointSize: radarPanel.theme.fontLead
                    font.bold: true
                    // Tabular figures. Proportional digits changed this label's
                    // width every frame, which resized the column and nudged the
                    // slider beside it once per animation step.
                    font.features: ({ tnum: 1 })
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
                    opacity: radarPanel.radarController.currentFrameObserved ? radarPanel.theme.emphasisLow : radarPanel.theme.emphasisFull
                    font.pointSize: radarPanel.theme.fontCaption
                    font.bold: true
                    font.letterSpacing: radarPanel.theme.capsTracking
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignRight
                }
            }
        }
    }
}
