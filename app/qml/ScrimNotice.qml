import QtQuick
import QtQuick.Controls

// A sentence laid over the radar, carrying the backing it needs to stay
// readable over whatever the frame beneath it happens to be painting. The
// scrim is a child rather than a wrapper: it fills the label and then grows one
// step past it on every side, which is where the padding comes from, and sits
// behind the text at z -1 so the label still measures and wraps as a label.
//
// Nothing here is declared that the Label already has. Where a notice sits,
// which colour it takes, and how many lines it may run to are the parts that
// differ between the two on the map, and all three are Label's or Item's own
// properties — so each call site says them in the ordinary way rather than
// through an alias renaming a property that already had a name. Defaulting
// `maximumLineCount` here would be the one that hurts: the two call sites
// disagree about it on purpose, and a shared default is how that disagreement
// would quietly become a single number.
Label {
    id: scrimNotice

    required property Theme theme

    // Both notices bind a string that is empty in the healthy case, so an empty
    // notice has to disappear entirely rather than leave a lit scrim over
    // nothing.
    visible: scrimNotice.text.length > 0
    width: Math.min(scrimNotice.implicitWidth, scrimNotice.parent.width - scrimNotice.theme.spaceXl * 2)
    font.pointSize: scrimNotice.theme.fontBody
    font.bold: true
    wrapMode: Text.Wrap
    elide: Text.ElideRight
    horizontalAlignment: Text.AlignHCenter

    Rectangle {
        anchors.fill: parent
        anchors.margins: -scrimNotice.theme.spaceSm
        color: scrimNotice.theme.background
        opacity: scrimNotice.theme.scrimOpacity
        radius: scrimNotice.theme.radiusSm
        z: -1
    }
}
