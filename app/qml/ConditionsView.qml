import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: conditionsView

    required property var weatherController
    required property Theme theme

    spacing: 10

    RowLayout {
        Layout.fillWidth: true
        spacing: 12

        Label {
            text: conditionsView.weatherController.stationName
            font.pointSize: 14
            font.bold: true
            font.letterSpacing: 0.5
            color: conditionsView.theme.text
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Label {
            id: observedLabel
            text: conditionsView.weatherController.observedLabel
            visible: observedLabel.text.length > 0
            font.pointSize: 10
            font.bold: true
            color: conditionsView.theme.text
            opacity: 0.7
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 12

        ConditionIcon {
            id: heroIcon
            source: conditionsView.weatherController.conditionIconUrl
            slotSize: 56
            Layout.preferredWidth: heroIcon.implicitWidth
            Layout.preferredHeight: heroIcon.implicitHeight
        }

        ColumnLayout {
            spacing: 0
            Layout.fillWidth: true

            Label {
                text: conditionsView.weatherController.temperature
                font.pointSize: 38
                font.bold: true
                color: conditionsView.theme.text
                Layout.fillWidth: true
                elide: Text.ElideRight
            }

            // Kept for the hero only, and kept to one line: at icon size the
            // 22 Buienradar codes are genuinely ambiguous, so the words earn
            // their space. It sits under the reading rather than beside it
            // because the longest of them ("Partly cloudy with light rain")
            // has no room left on that row without wrapping or eliding.
            Label {
                text: conditionsView.weatherController.condition
                font.pointSize: 13
                color: conditionsView.theme.text
                Layout.fillWidth: true
                maximumLineCount: 1
                elide: Text.ElideRight
            }
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: metricsRow.implicitHeight + 20
        color: conditionsView.theme.panel
        radius: 8

        RowLayout {
            id: metricsRow
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 14
            anchors.rightMargin: 14
            spacing: 12

            MetricChip {
                theme: conditionsView.theme
                label: "WIND"
                value: conditionsView.weatherController.wind
                Layout.fillWidth: true
            }
            MetricChip {
                theme: conditionsView.theme
                label: "FEELS"
                value: conditionsView.weatherController.feelsLike
                Layout.fillWidth: true
            }
            MetricChip {
                theme: conditionsView.theme
                label: "HUMID"
                value: conditionsView.weatherController.humidity
                Layout.fillWidth: true
            }
        }
    }

    Label {
        id: conditionsError
        objectName: "conditionsError"
        text: conditionsView.weatherController.status
        visible: conditionsError.text.length > 0
        color: conditionsView.theme.error
        Layout.fillWidth: true
        wrapMode: Text.Wrap
    }
}
