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

    // The five bucket ranges sit a step under the unit that governs them: they
    // are the key's data, the unit is its title, and at one shared size the
    // block read as six equal captions. One point is the whole difference —
    // enough to rank them, not enough to make the digits work to read. A
    // seventh entry in the type scale would mean two things competing at the
    // same rank, so a size used in exactly one place stays local to that file.
    readonly property int legendRangeSize: radarPanel.theme.fontCaption - 1

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

                    // Equal cells, and every delegate has to fill the cell it is
                    // handed. Both halves are load-bearing: without the
                    // `fillWidth` below, a column sizes to its own caption and
                    // centres inside its cell, so the swatch above it inherits
                    // the width of a string. That is how "10-100" came to draw
                    // twice as wide as "0-2" — a difference that encoded nothing
                    // but character count, on a key whose whole job is to say
                    // that the buckets are read by colour and not by width.
                    uniformCellSizes: true

                    Repeater {
                        model: radarPanel.radarController.legendSwatches

                        ColumnLayout {
                            id: bucket
                            required property int index
                            required property var modelData

                            Layout.fillWidth: true
                            // Equal cells alone are not enough: the caption sets
                            // this column's implicit width but `fillWidth` lets
                            // it shrink under that, and the captions do not
                            // elide, so a cell narrower than its text spills the
                            // text over its neighbour. Flooring each cell at its
                            // own caption makes the widest one ("10-100") the
                            // floor for all five, which is also what widens the
                            // card enough to hold them.
                            Layout.minimumWidth: rangeCaption.implicitWidth
                            spacing: 2

                            Rectangle {
                                Layout.fillWidth: true
                                implicitHeight: radarPanel.theme.spaceSm
                                radius: radarPanel.theme.radiusSm
                                color: bucket.modelData
                            }

                            Label {
                                id: rangeCaption
                                text: radarPanel.radarController.legendBuckets[bucket.index]
                                font.pointSize: radarPanel.legendRangeSize
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

        // Not the error slot, and deliberately not centred with it. A location
        // outside the Dutch raster leaves this map painting its base and nothing
        // else, which is indistinguishable from a failed frame request — so the
        // one thing worth saying is that the emptiness is the configuration
        // rather than a fault. It is a standing statement, not a failure, so it
        // takes the text colour instead of the error colour and sits at the top,
        // where it cannot collide with a refresh error arriving underneath it.
        ScrimNotice {
            objectName: "coverageNotice"
            theme: radarPanel.theme
            text: radarPanel.radarController.coverageNotice
            color: radarPanel.theme.text
            maximumLineCount: 2
            anchors.top: parent.top
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.topMargin: radarPanel.theme.spaceLg
        }

        ScrimNotice {
            objectName: "radarError"
            theme: radarPanel.theme
            text: radarPanel.radarController.status
            color: radarPanel.theme.error
            maximumLineCount: 3
            anchors.centerIn: parent
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

            // The transport verb is drawn instead of typed. "Play" and "Pause"
            // are words a reader has to stop and parse; the two marks that every
            // player on the desktop already uses say the same thing without being
            // read, and dropping the word gives the timeline beside it the ~44 px
            // the label was holding — which is the whole reason this control is
            // now 36 square rather than 80 wide. The word itself does not leave:
            // `text:` stays because `AbstractButton.text` is this button's
            // accessible name, and a screen reader still has to be able to say
            // "Pause" after the pixels stopped saying it.
            //
            // The glyph is a plain CHILD of the button carrying its own explicit
            // size, not the button's `contentItem`. A Control sizes its content
            // item to `availableWidth`, and at this width Material's padding
            // leaves nothing of it — measured in this app at 36 - 24 - 24, floored
            // to zero — so a Canvas mounted there has no area to paint into and
            // stays empty however often it is asked to draw. A child is not sized
            // by the Control at all: it keeps its 12 px and centres on the button.
            // The repaint hooks are a guarantee rather than a diagnosis — the
            // glyph asks to be redrawn on each input it reads and once more when
            // the surface reports itself paintable, so the first paint does not
            // depend on which of those comes first.
            Button {
                id: playButton
                text: radarPanel.radarController.playing ? "Pause" : "Play"
                enabled: radarPanel.radarController.frameCount > 1
                Layout.preferredHeight: 36
                Layout.preferredWidth: 36

                background: Rectangle {
                    color: playButton.down ? radarPanel.theme.selection : radarPanel.theme.buttonFill
                    radius: radarPanel.theme.radiusSm
                }
                contentItem: Item {}

                Canvas {
                    id: transportGlyph
                    readonly property bool showPause: radarPanel.radarController.playing
                    readonly property color ink: radarPanel.theme.buttonText
                    readonly property int side: 12
                    anchors.centerIn: parent
                    z: 2
                    width: transportGlyph.side
                    height: transportGlyph.side
                    onShowPauseChanged: transportGlyph.requestPaint()
                    onInkChanged: transportGlyph.requestPaint()
                    onWidthChanged: transportGlyph.requestPaint()
                    onHeightChanged: transportGlyph.requestPaint()
                    Component.onCompleted: transportGlyph.requestPaint()
                    onAvailableChanged: if (transportGlyph.available) transportGlyph.requestPaint()
                    onPaint: {
                        const ctx = transportGlyph.getContext("2d");
                        ctx.clearRect(0, 0, transportGlyph.width, transportGlyph.height);
                        ctx.fillStyle = transportGlyph.ink;
                        const left = (transportGlyph.width - transportGlyph.side) / 2;
                        const top = (transportGlyph.height - transportGlyph.side) / 2;
                        if (transportGlyph.showPause) {
                            const bar = transportGlyph.side / 3;
                            ctx.fillRect(left, top, bar, transportGlyph.side);
                            ctx.fillRect(left + transportGlyph.side - bar, top, bar, transportGlyph.side);
                        } else {
                            ctx.beginPath();
                            ctx.moveTo(left + 1, top);
                            ctx.lineTo(left + transportGlyph.side, top + transportGlyph.side / 2);
                            ctx.lineTo(left + 1, top + transportGlyph.side);
                            ctx.closePath();
                            ctx.fill();
                        }
                    }
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
