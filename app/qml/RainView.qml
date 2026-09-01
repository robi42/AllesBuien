// Delegates below reach the component root's id for its injected `theme`.
// Bound component behaviour is what makes that access resolvable instead of
// a late context lookup; every model role in this file is a required property.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: rainView

    required property var weatherController
    required property Theme theme

    // Bars are inset from their cell by this much, so the gap between two bars
    // is one spacing step whatever the sample count turns out to be.
    readonly property int barGap: rainView.theme.spaceXs
    readonly property int axisHeight: rainView.theme.spaceLg

    spacing: rainView.theme.spaceSm

    RowLayout {
        Layout.fillWidth: true
        spacing: rainView.theme.spaceMd

        Label {
            text: "Rain expectation"
            font.pointSize: rainView.theme.fontLabel
            font.bold: true
            color: rainView.theme.text
            opacity: rainView.theme.emphasisLow
        }

        Label {
            text: rainView.weatherController.rainSummary
            font.pointSize: rainView.theme.fontLabel
            font.bold: true
            color: rainView.weatherController.rainWet ? rainView.theme.accent : rainView.theme.text
            opacity: rainView.weatherController.rainWet ? rainView.theme.emphasisFull : rainView.theme.emphasisLow
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignRight
            elide: Text.ElideRight
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 48
        color: rainView.theme.panel
        radius: rainView.theme.radiusMd

        Item {
            id: plot
            anchors.fill: parent
            anchors.margins: rainView.theme.spaceSm

            readonly property int cellCount: Math.max(1, rainView.weatherController.rainLevels.length)
            readonly property real cellWidth: plot.width / plot.cellCount

            Repeater {
                model: rainView.weatherController.rainLevels

                Rectangle {
                    id: bar
                    required property int index
                    required property var modelData

                    x: bar.index * plot.cellWidth
                    width: Math.max(2, plot.cellWidth - rainView.barGap)
                    y: plot.height - bar.height
                    height: Math.max(2, plot.height * bar.modelData)
                    radius: rainView.theme.radiusSm
                    color: rainView.theme.accent
                    opacity: bar.modelData > 0 ? rainView.theme.emphasisFull : 0.45
                }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: rainView.theme.hairline
                color: rainView.theme.text
                opacity: 0.4
            }

            // The moment the radar is showing, so the map and the graph read as
            // one timeline. Rust returns no fraction for a frame outside the
            // series — the observed frames before it starts and the predictions
            // that outrun it — and the cursor hides rather than lying at an edge.
            Rectangle {
                id: radarCursor
                x: rainView.weatherController.rainCursorFraction * plot.width - radarCursor.width / 2
                y: 0
                width: 2
                height: plot.height
                color: rainView.theme.text
                opacity: 0.75
                visible: rainView.weatherController.rainCursorVisible
            }
        }
    }

    Item {
        Layout.fillWidth: true
        Layout.preferredHeight: rainView.axisHeight

        Item {
            id: axis
            anchors.fill: parent
            anchors.leftMargin: rainView.theme.spaceSm
            anchors.rightMargin: rainView.theme.spaceSm

            Repeater {
                model: rainView.weatherController.rainAxisLabels

                Label {
                    id: axisLabel
                    required property int index
                    required property var modelData

                    text: axisLabel.modelData
                    visible: axisLabel.text.length > 0
                    font.pointSize: rainView.theme.fontCaption
                    font.features: ({ tnum: 1 })
                    color: rainView.theme.text
                    opacity: rainView.theme.emphasisLow
                    x: Math.max(0, Math.min(axis.width - axisLabel.width, (axisLabel.index + 0.5) * plot.cellWidth - axisLabel.width / 2))
                }
            }
        }
    }
}
