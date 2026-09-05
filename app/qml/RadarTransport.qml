import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: radarTransport

    required property var radarController
    required property Theme theme

    // Used on both axes of the handle; every other one-off size below is written
    // where it is used, and anything that lands on the 4 px scale comes from the
    // theme rather than being restated here.
    readonly property int handleSize: 18

    anchors.bottom: parent.bottom
    anchors.left: parent.left
    anchors.right: parent.right
    height: 56
    color: radarTransport.theme.surface
    radius: radarTransport.theme.radiusLg

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: radarTransport.theme.spaceLg
        anchors.rightMargin: radarTransport.theme.spaceXl
        spacing: radarTransport.theme.spaceXl

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
            text: radarTransport.radarController.playing ? "Pause" : "Play"
            enabled: radarTransport.radarController.frameCount > 1
            Layout.preferredHeight: 36
            Layout.preferredWidth: 36

            background: Rectangle {
                color: playButton.down ? radarTransport.theme.selection : radarTransport.theme.buttonFill
                radius: radarTransport.theme.radiusSm
            }
            contentItem: Item {}

            Canvas {
                id: transportGlyph
                readonly property bool showPause: radarTransport.radarController.playing
                readonly property color ink: radarTransport.theme.buttonText
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

            onClicked: radarTransport.radarController.playing ? radarTransport.radarController.pause() : radarTransport.radarController.play()
        }

        Slider {
            id: frameSlider
            Layout.fillWidth: true
            from: 0
            to: Math.max(0, radarTransport.radarController.frameCount - 1)
            value: radarTransport.radarController.currentFrameIndex
            stepSize: 1
            enabled: radarTransport.radarController.frameCount > 1

            background: Rectangle {
                id: sliderGroove
                x: frameSlider.leftPadding
                y: frameSlider.topPadding + frameSlider.availableHeight / 2 - sliderGroove.height / 2
                implicitWidth: 200
                implicitHeight: 6
                width: frameSlider.availableWidth
                height: sliderGroove.implicitHeight
                radius: sliderGroove.height / 2
                color: radarTransport.theme.sliderTrack

                // Where measurement stops and prediction starts. The window
                // is 2 observations against 24 predictions, so a lone hairline
                // this far left was easy to miss; the travelled part of the
                // track changes colour at the same x instead, which makes the
                // boundary an edge rather than something to hunt for.
                // Clamping is what handles Rust's degenerate runs: an
                // all-forecast run reports a fraction below 0 and comes out
                // fully accent, an all-observed one reports above 1 and comes
                // out fully muted, and neither draws the divider.
                readonly property real boundaryX: Math.max(0, Math.min(1, radarTransport.radarController.observedFraction)) * sliderGroove.width
                readonly property real travelledX: frameSlider.visualPosition * sliderGroove.width
                readonly property real observedX: Math.min(sliderGroove.travelledX, sliderGroove.boundaryX)

                Rectangle {
                    width: sliderGroove.observedX
                    height: sliderGroove.height
                    radius: sliderGroove.radius
                    color: radarTransport.theme.text
                    opacity: 0.55
                }

                Rectangle {
                    x: sliderGroove.observedX
                    width: Math.max(0, sliderGroove.travelledX - sliderGroove.observedX)
                    height: sliderGroove.height
                    radius: sliderGroove.radius
                    color: radarTransport.theme.accent
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
                    color: radarTransport.theme.background
                    border.width: 1
                    border.color: radarTransport.theme.text
                    visible: radarTransport.radarController.forecastBoundaryVisible
                }
            }
            handle: Rectangle {
                id: sliderHandle
                x: frameSlider.leftPadding + frameSlider.visualPosition * (frameSlider.availableWidth - sliderHandle.width)
                y: frameSlider.topPadding + frameSlider.availableHeight / 2 - sliderHandle.height / 2
                implicitWidth: radarTransport.handleSize
                implicitHeight: radarTransport.handleSize
                radius: sliderHandle.width / 2
                color: radarTransport.theme.accent
                border.width: 2
                border.color: radarTransport.theme.surface
            }

            onMoved: radarTransport.radarController.seek(Math.round(frameSlider.value))
        }

        ColumnLayout {
            spacing: 0
            Layout.minimumWidth: 84

            Label {
                text: radarTransport.radarController.currentFrameLabel
                color: radarTransport.theme.text
                font.pointSize: radarTransport.theme.fontLead
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
                text: radarTransport.radarController.currentFrameObserved ? "OBSERVED" : "FORECAST"
                visible: radarTransport.radarController.frameCount > 0
                color: radarTransport.theme.text
                opacity: radarTransport.radarController.currentFrameObserved ? radarTransport.theme.emphasisLow : radarTransport.theme.emphasisFull
                font.pointSize: radarTransport.theme.fontCaption
                font.bold: true
                font.letterSpacing: radarTransport.theme.capsTracking
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignRight
            }
        }
    }
}
