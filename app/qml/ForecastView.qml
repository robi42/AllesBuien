// Delegates below reach the component root's id for its injected `theme`.
// Bound component behaviour is what makes that access resolvable instead of
// a late context lookup; every model role in this file is a required property.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ListView {
    id: forecastView

    required property var weatherController
    required property Theme theme

    // Three columns, fixed width, identical in every row — which is the whole
    // point: five rows only scan as a table if their column edges line up.
    // The middle column is one group (icon plus chance) of fixed total width,
    // centred in the elastic remainder, so the free space falls symmetrically
    // on both sides of it instead of pooling into a single hole between a
    // left cluster and a right one.
    readonly property int dayWidth: 44
    readonly property int rainWidth: 52
    readonly property int temperatureWidth: 92
    // Rows divide the view's height between them; the floor only matters on a
    // short window, where the list clips rather than squashing the rows.
    readonly property int rowMinHeight: 40

    clip: true
    model: forecastView.weatherController
    interactive: false

    delegate: Item {
        id: forecastRow

        required property var weekday
        required property var iconUrl
        required property var rainChance
        required property var minTemperature
        required property var maxTemperature

        width: forecastView.width
        height: Math.max(forecastView.rowMinHeight, forecastView.height / Math.max(1, forecastView.count))

        RowLayout {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: forecastView.theme.spaceMd

            Label {
                text: forecastRow.weekday
                font.pointSize: forecastView.theme.fontValue
                font.bold: true
                color: forecastView.theme.text
                Layout.preferredWidth: forecastView.dayWidth
            }

            // The elastic column. Its content has a fixed width, so the icon and
            // the chance sit at the same x in all five rows.
            RowLayout {
                Layout.fillWidth: true
                spacing: forecastView.theme.spaceSm

                Item {
                    Layout.fillWidth: true
                }

                ConditionIcon {
                    id: rowIcon
                    source: forecastRow.iconUrl
                    slotSize: forecastView.theme.iconRow
                    Layout.preferredWidth: rowIcon.implicitWidth
                    Layout.preferredHeight: rowIcon.implicitHeight
                }

                // A nested layout defaults to fillWidth, which would let this
                // cell swallow the free space and strand the dot beside the
                // icon. Fixed width is the whole point: the chance has to land
                // on the same x in every row.
                RowLayout {
                    Layout.fillWidth: false
                    Layout.preferredWidth: forecastView.rainWidth
                    spacing: forecastView.theme.spaceXs

                    Rectangle {
                        visible: forecastRow.rainChance > 0
                        radius: forecastView.theme.radiusSm
                        color: forecastView.theme.accent
                        Layout.preferredWidth: forecastView.theme.spaceSm
                        Layout.preferredHeight: forecastView.theme.spaceSm
                    }
                    Label {
                        text: forecastRow.rainChance > 0 ? (forecastRow.rainChance + "%") : ""
                        font.pointSize: forecastView.theme.fontLabel
                        font.bold: true
                        font.features: ({ tnum: 1 })
                        color: forecastView.theme.accent
                        Layout.fillWidth: true
                        horizontalAlignment: Text.AlignRight
                    }
                }

                Item {
                    Layout.fillWidth: true
                }
            }

            // Tabular figures keep the slash and both readings on the same x
            // down the column, whatever digits a day happens to carry.
            Label {
                text: forecastRow.minTemperature + "° / " + forecastRow.maxTemperature + "°"
                font.pointSize: forecastView.theme.fontValue
                font.bold: true
                font.features: ({ tnum: 1 })
                color: forecastView.theme.text
                Layout.preferredWidth: forecastView.temperatureWidth
                horizontalAlignment: Text.AlignRight
            }
        }

        // The rule is what turns five rows into a table, so it has to actually
        // reach the eye: `rowSeparator` differs from `surface` by 30 across the
        // channels, and at 0.4 that landed under a 12-per-channel detection
        // threshold — visible to a scan, not to a reader.
        Rectangle {
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            height: forecastView.theme.hairline
            color: forecastView.theme.rowSeparator
            opacity: 0.6
        }
    }

    Label {
        id: forecastError
        objectName: "forecastError"
        text: forecastView.weatherController.status
        visible: forecastView.count === 0 && forecastError.text.length > 0
        anchors.centerIn: parent
        width: parent.width - forecastView.theme.spaceLg
        color: forecastView.theme.error
        wrapMode: Text.Wrap
        horizontalAlignment: Text.AlignHCenter
    }
}
