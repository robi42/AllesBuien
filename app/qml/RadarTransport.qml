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
    // The keyboard-focus ring, on both controls: a border on the play button's
    // background, and the handle's own border changing colour on the slider.
    readonly property int focusRingWidth: 2

    // Rust's `play` already refuses a run of one frame; this is the same rule
    // stated once for everything here that has to go inert with it — both
    // controls and every key below.
    readonly property bool playable: radarTransport.radarController.frameCount > 1

    // Whether the focus in this transport last arrived from the keyboard. The
    // rings draw on this together with `activeFocus` rather than on Controls'
    // own `visualFocus`, which only lasts until the next radar refresh: a
    // refresh empties the run while it fetches, both controls disable, and Qt
    // hands the focus back when they re-enable, but under a reason that is not
    // a keyboard one, so `visualFocus` comes back false (measured). Only Tab,
    // Shift+Tab, and a click write this latch, so it rides out that round trip
    // and a keyboard user keeps the ring while a mouse user never gets one. A
    // click on the control that already has the focus moves nothing and keeps
    // its ring, as `visualFocus` does.
    property bool keyboardFocus: false

    // How the focused control got its focus, or -1 while neither has it. One
    // reading of whichever control holds the focus, rather than a handler on
    // each, because a move between the two reports its reason on both sides.
    readonly property int controlFocusReason: playButton.activeFocus ? playButton.focusReason : frameSlider.activeFocus ? frameSlider.focusReason : -1

    onControlFocusReasonChanged: {
        const reason = radarTransport.controlFocusReason;
        if (reason === Qt.TabFocusReason || reason === Qt.BacktabFocusReason || reason === Qt.ShortcutFocusReason)
            radarTransport.keyboardFocus = true;
        else if (reason === Qt.MouseFocusReason)
            radarTransport.keyboardFocus = false;
    }

    function togglePlayback(): void {
        if (radarTransport.radarController.playing)
            radarTransport.radarController.pause();
        else
            radarTransport.radarController.play();
    }

    anchors.bottom: parent.bottom
    anchors.left: parent.left
    anchors.right: parent.right
    height: 56
    color: radarTransport.theme.surface
    radius: radarTransport.theme.radiusLg

    // The playback keys, wherever the focus is in the window. They are
    // window-context Shortcuts rather than `Keys` handlers because Qt consults
    // its shortcut map before it delivers a press to the focused item, and
    // neither control here claims the press first: the focused Button would
    // otherwise click on Space, Return, and Enter itself, and the focused
    // Slider would step on Left and Right, so a handler-based scheme toggles
    // twice or never depending on where the focus sits. A Shortcut pre-empts
    // instead, so every key has one meaning and this list is all of them.
    //
    // A held Space is one toggle; a held arrow scrubs. Up and Down stay
    // unbound: the timeline is horizontal, and they would only be a second
    // spelling of Left and Right. Tab and Shift+Tab need nothing here — the
    // play button and the slider are the window's only tab stops, in this
    // file's item order.
    Shortcut {
        sequences: ["Space", "Return", "Enter"]
        enabled: radarTransport.playable
        autoRepeat: false
        onActivated: radarTransport.togglePlayback()
    }

    Shortcut {
        sequences: ["Left"]
        enabled: radarTransport.playable
        onActivated: radarTransport.radarController.seek(radarTransport.radarController.currentFrameIndex - 1)
    }

    Shortcut {
        sequences: ["Right"]
        enabled: radarTransport.playable
        onActivated: radarTransport.radarController.seek(radarTransport.radarController.currentFrameIndex + 1)
    }

    Shortcut {
        sequences: ["Home"]
        enabled: radarTransport.playable
        onActivated: radarTransport.radarController.seek(0)
    }

    Shortcut {
        sequences: ["End"]
        enabled: radarTransport.playable
        onActivated: radarTransport.radarController.seek(radarTransport.radarController.frameCount - 1)
    }

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
            objectName: "playButton"
            text: radarTransport.radarController.playing ? "Pause" : "Play"
            enabled: radarTransport.playable
            Layout.preferredHeight: 36
            Layout.preferredWidth: 36

            background: Rectangle {
                color: playButton.down ? radarTransport.theme.selection : radarTransport.theme.buttonFill
                radius: radarTransport.theme.radiusSm
                border.width: playButton.activeFocus && radarTransport.keyboardFocus ? radarTransport.focusRingWidth : 0
                border.color: radarTransport.theme.focusRing
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

            onClicked: radarTransport.togglePlayback()
        }

        Slider {
            id: frameSlider
            objectName: "frameSlider"
            Layout.fillWidth: true
            from: 0
            to: Math.max(0, radarTransport.radarController.frameCount - 1)
            value: radarTransport.radarController.currentFrameIndex
            stepSize: 1
            enabled: radarTransport.playable

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
                // is a couple of observations against the two dozen or so
                // predictions up to the rain graph's last bar, so a lone
                // hairline this far left was easy to miss; the travelled part
                // of the track changes colour at the same x instead, which
                // makes the boundary an edge rather than something to hunt for.
                // Rust already pins the degenerate runs to the ends: an
                // all-forecast run reports 0 and comes out fully accent, an
                // all-observed one reports 1 and comes out fully muted, and
                // neither draws the divider. The clamp only guards the binding.
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
            // The surface-coloured border is a gap that lifts the handle off
            // the track; under keyboard focus the same border becomes the ring.
            handle: Rectangle {
                id: sliderHandle
                objectName: "sliderHandle"
                x: frameSlider.leftPadding + frameSlider.visualPosition * (frameSlider.availableWidth - sliderHandle.width)
                y: frameSlider.topPadding + frameSlider.availableHeight / 2 - sliderHandle.height / 2
                implicitWidth: radarTransport.handleSize
                implicitHeight: radarTransport.handleSize
                radius: sliderHandle.width / 2
                color: radarTransport.theme.accent
                border.width: radarTransport.focusRingWidth
                border.color: frameSlider.activeFocus && radarTransport.keyboardFocus ? radarTransport.theme.focusRing : radarTransport.theme.surface
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
