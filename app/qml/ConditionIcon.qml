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

    // The pending icon's placeholder, in the inset tone the metrics panel
    // already uses, not an invented colour. It used to be `surface`, which is
    // the colour of the card both call sites sit on, so the skeleton was drawn
    // on every cold load and seen on none of them.
    Rectangle {
        id: skeleton
        objectName: "skeleton"
        anchors.fill: parent
        radius: conditionIcon.theme.radiusMd
        color: conditionIcon.theme.panel
        opacity: conditionIcon.theme.emphasisLow
        visible: conditionIcon.loading && iconImage.status !== Image.Ready

        // Shaped like the radar loading notice's pulse, for the reasons given
        // there: a 2 s breath between two emphasis steps the theme already
        // carries. `running` binds the skeleton's own visibility, because an
        // animation left running behind an invisible item is a battery cost
        // that nothing on screen can account for.
        SequentialAnimation on opacity {
            running: skeleton.visible && conditionIcon.pulse
            loops: Animation.Infinite

            NumberAnimation {
                from: conditionIcon.theme.emphasisFull
                to: conditionIcon.theme.emphasisLow
                duration: 1000
                easing.type: Easing.InOutSine
            }

            NumberAnimation {
                from: conditionIcon.theme.emphasisLow
                to: conditionIcon.theme.emphasisFull
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
