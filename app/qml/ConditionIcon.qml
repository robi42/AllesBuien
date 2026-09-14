import QtQuick

Item {
    id: conditionIcon

    property alias source: iconImage.source
    // Required rather than defaulted: the two call sites use different sizes and
    // a default would silently hide one of them behind the other's number.
    required property int slotSize
    required property Theme theme
    // Required for the same reason as `slotSize`: an empty source means either
    // "still downloading" or "never coming", and only the controller knows which.
    // A skeleton that stayed up for a failed download would be worse than the
    // empty slot it replaces, so no call site may leave this to a default.
    required property bool loading
    // Only the hero breathes. Five row skeletons pulsing together read as noise.
    property bool pulse: false

    // The slot is reserved unconditionally. Sizing it from the image status
    // collapsed it to zero until the icon resolved, so every row it sat in
    // snapped sideways the moment the icons arrived. An icon that never
    // loads now leaves the reserved space empty instead of moving the text.
    implicitWidth: conditionIcon.slotSize
    implicitHeight: conditionIcon.slotSize

    // How strongly the skeleton tints its card with the theme's text colour.
    // Named here because nothing else draws with them. A tint of `text` is what
    // keeps the placeholder visible on every theme: `text` contrasts with the
    // card by construction, light or dark, while any second surface tone may
    // sit a handful of levels from it. The first skeleton used `surface`, the
    // card's own colour, and then `panel`, whose full pulse on Nord moved four
    // levels per channel; both rendered and neither could be seen. At these
    // strengths Nord's card moves 16 and 41 levels in red, Flexoki Light's 21
    // and 56, above the 12-per-channel threshold `ForecastView` measured.
    readonly property real skeletonFaint: 0.1
    readonly property real skeletonStrong: 0.26

    Rectangle {
        id: skeleton
        objectName: "skeleton"
        anchors.fill: parent
        radius: conditionIcon.theme.radiusMd
        color: conditionIcon.theme.text
        opacity: conditionIcon.skeletonStrong
        visible: conditionIcon.loading && iconImage.status !== Image.Ready

        // Timed like the radar loading notice's pulse, for the reasons given
        // there: a 2 s breath with sine easing reads as waiting, not alarm.
        // `running` binds the skeleton's own visibility, because an animation
        // left running behind an invisible item is a battery cost that nothing
        // on screen can account for.
        SequentialAnimation on opacity {
            running: skeleton.visible && conditionIcon.pulse
            loops: Animation.Infinite

            NumberAnimation {
                from: conditionIcon.skeletonStrong
                to: conditionIcon.skeletonFaint
                duration: 1000
                easing.type: Easing.InOutSine
            }

            NumberAnimation {
                from: conditionIcon.skeletonFaint
                to: conditionIcon.skeletonStrong
                duration: 1000
                easing.type: Easing.InOutSine
            }
        }
    }

    Image {
        id: iconImage
        anchors.fill: parent
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        // Hidden unless decoded, so a failed load is blank, never a broken-image box.
        visible: iconImage.status === Image.Ready
    }
}
