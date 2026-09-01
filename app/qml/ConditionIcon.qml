import QtQuick

Item {
    id: conditionIcon

    property alias source: iconImage.source
    // Required rather than defaulted: the two call sites use different sizes and
    // a default would silently hide one of them behind the other's number.
    required property int slotSize

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
