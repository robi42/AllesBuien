// Delegates below reach the component root's id for its injected `theme`.
// Bound component behaviour is what makes that access resolvable instead of
// a late context lookup; every model role in this file is a required property.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

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
    id: radarLegend

    required property var radarController
    required property Theme theme

    // The five bucket ranges sit under the unit that governs them: they are the
    // key's data, the unit is its title, and at one shared size the block read
    // as six equal captions. It sat one point under until the longest range was
    // measured against its own colour block: "10-100" came to 42.75 px in a
    // 43 px cell, so its digits ran edge to edge with the block they label —
    // nothing between glyph and boundary, against 13 px of air under "0-2".
    // The widest label was setting the whole key's geometry and had no margin
    // of its own, which is the one caption a reader has to work hardest at.
    //
    // Two points is what buys that margin back. Dropping the point alone does
    // not: the floor below tracks the caption, so a smaller string simply gets
    // a smaller cell and the clearance stays at zero — measured, and it came
    // out marginally worse. The point is only worth spending alongside the
    // reserved margin the floor now adds, and together they cost nothing: the
    // card measures the same 255 px it did before and the five blocks are
    // unchanged, while the longest range gained 4 px of air on either side.
    // A seventh entry in the type scale would mean two things competing at the
    // same rank, so a size used in exactly one place stays local to this file.
    readonly property int legendRangeSize: radarLegend.theme.fontCaption - 2

    anchors.left: parent.left
    anchors.bottom: parent.bottom
    anchors.margins: radarLegend.theme.spaceMd
    implicitWidth: legendColumn.implicitWidth + radarLegend.theme.spaceMd * 2
    implicitHeight: legendColumn.implicitHeight + radarLegend.theme.spaceMd * 2
    radius: radarLegend.theme.radiusMd
    color: radarLegend.theme.surface
    visible: radarLegend.radarController.frameCount > 0

    ColumnLayout {
        id: legendColumn
        anchors.fill: parent
        anchors.margins: radarLegend.theme.spaceMd
        spacing: radarLegend.theme.spaceXs

        Label {
            text: radarLegend.radarController.legendUnit
            font.pointSize: radarLegend.theme.fontCaption
            font.bold: true
            font.letterSpacing: radarLegend.theme.capsTracking
            color: radarLegend.theme.text
            opacity: radarLegend.theme.emphasisMid
        }

        RowLayout {
            spacing: radarLegend.theme.spaceXs

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
                model: radarLegend.radarController.legendSwatches

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
                    //
                    // The margin is part of the floor rather than an
                    // afterthought, because a floor set at the bare text
                    // width is self-defeating: it hands the widest
                    // caption a cell exactly as wide as its own glyphs,
                    // at every font size, so that one label alone never
                    // gets the air the other four have. Reserving a step
                    // on each side is what makes the cell a block the
                    // caption sits inside instead of a cast of its
                    // longest string.
                    Layout.minimumWidth: rangeCaption.implicitWidth
                        + radarLegend.theme.spaceXs * 2
                    spacing: 2

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: radarLegend.theme.spaceSm
                        radius: radarLegend.theme.radiusSm
                        color: bucket.modelData
                    }

                    Label {
                        id: rangeCaption
                        text: radarLegend.radarController.legendBuckets[bucket.index]
                        font.pointSize: radarLegend.legendRangeSize
                        font.features: ({ tnum: 1 })
                        color: radarLegend.theme.text
                        opacity: radarLegend.theme.emphasisMid
                        Layout.fillWidth: true
                        horizontalAlignment: Text.AlignHCenter
                    }
                }
            }
        }
    }
}
