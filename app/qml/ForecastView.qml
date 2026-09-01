// Delegates below reach the component root's id for its injected `theme`.
// Bound component behaviour is what makes that access resolvable instead of
// a late context lookup; every model role in this file is a required property.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ListView {
    id: forecastList

    required property var weatherController
    required property Theme theme

    clip: true
    model: forecastList.weatherController
    interactive: false

    delegate: Item {
        id: forecastRow

        required property var weekday
        required property var iconUrl
        required property var rainChance
        required property var minTemperature
        required property var maxTemperature

        width: ListView.view.width
        height: Math.max(44, rowLayout.implicitHeight + 12)

        RowLayout {
            id: rowLayout
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 4
            anchors.rightMargin: 4
            spacing: 10

            Label {
                text: forecastRow.weekday
                Layout.preferredWidth: 40
                font.pointSize: 12
                font.bold: true
                color: forecastList.theme.text
            }

            ConditionIcon {
                id: rowIcon
                source: forecastRow.iconUrl
                Layout.preferredWidth: rowIcon.implicitWidth
                Layout.preferredHeight: rowIcon.implicitHeight
            }

            // The condition sentence used to live here and was the widest
            // column, wrapping most rows onto two lines. Icon, chance, and
            // high/low carry the same meaning in one line; this spacer takes
            // the width it freed and anchors both numeric columns right.
            Item {
                Layout.fillWidth: true
            }

            RowLayout {
                Layout.preferredWidth: 52
                spacing: 4
                Rectangle {
                    Layout.preferredWidth: 8
                    Layout.preferredHeight: 8
                    radius: 4
                    color: forecastList.theme.accent
                    visible: forecastRow.rainChance > 0
                }
                Label {
                    text: forecastRow.rainChance > 0 ? (forecastRow.rainChance + "%") : ""
                    font.pointSize: 11
                    font.bold: true
                    color: forecastList.theme.accent
                }
            }

            Label {
                text: forecastRow.minTemperature + "° / " + forecastRow.maxTemperature + "°"
                Layout.preferredWidth: 74
                font.pointSize: 12
                font.bold: true
                color: forecastList.theme.text
                horizontalAlignment: Text.AlignRight
            }
        }

        Rectangle {
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            height: 1
            color: forecastList.theme.rowSeparator
            opacity: 0.3
        }
    }

    Label {
        id: forecastError
        objectName: "forecastError"
        anchors.centerIn: parent
        text: forecastList.weatherController.status
        visible: forecastList.count === 0 && forecastError.text.length > 0
        color: forecastList.theme.error
        width: parent.width - 16
        wrapMode: Text.Wrap
        horizontalAlignment: Text.AlignHCenter
    }
}
