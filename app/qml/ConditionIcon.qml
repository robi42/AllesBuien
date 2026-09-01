import QtQuick

Item {
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
