import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Three ranks, and only three. Primary is the reading — it is the largest thing
// in the window and nothing else comes near it. Secondary is what the reading
// means: the condition sentence and the three measured values. Tertiary is every
// name and caption, all of them at one caption size, uppercase and tracked, so
// they read as field names rather than as competing headlines.
ColumnLayout {
    id: conditionsView

    required property var weatherController
    required property Theme theme

    spacing: conditionsView.theme.spaceMd

    // Tertiary: the place the app was asked about. It was a 14 pt bold headline
    // and was the second-largest thing on screen for a value the user chose and
    // already knows; as a tracked caption it labels the panel without competing
    // with the reading below it.
    Label {
        text: conditionsView.weatherController.locationName.toUpperCase()
        font.pointSize: conditionsView.theme.fontLabel
        font.bold: true
        font.letterSpacing: conditionsView.theme.capsTracking
        color: conditionsView.theme.text
        opacity: conditionsView.theme.emphasisMid
        Layout.fillWidth: true
        elide: Text.ElideRight
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: conditionsView.theme.spaceMd

        ConditionIcon {
            id: heroIcon
            source: conditionsView.weatherController.conditionIconUrl
            slotSize: conditionsView.theme.iconHero
            Layout.preferredWidth: heroIcon.implicitWidth
            Layout.preferredHeight: heroIcon.implicitHeight
        }

        ColumnLayout {
            spacing: 0
            Layout.fillWidth: true

            // Primary.
            Label {
                text: conditionsView.weatherController.temperature
                font.pointSize: conditionsView.theme.fontHero
                font.bold: true
                // Tabular figures, for the same reason the frame clock has them:
                // this label is rewritten on every refresh and proportional
                // digits would change its width, and the icon's, as it lands.
                font.features: ({ tnum: 1 })
                color: conditionsView.theme.text
                Layout.fillWidth: true
                elide: Text.ElideRight
            }

            // Secondary. Kept for the hero only, and kept to one line: at icon
            // size the 22 Buienradar codes are genuinely ambiguous, so the words
            // earn their space. It sits under the reading rather than beside it
            // because the longest of them ("Partly cloudy with light rain") has
            // no room left on that row without wrapping or eliding.
            Label {
                text: conditionsView.weatherController.condition
                font.pointSize: conditionsView.theme.fontBody
                color: conditionsView.theme.text
                opacity: conditionsView.theme.emphasisMid
                Layout.fillWidth: true
                maximumLineCount: 1
                elide: Text.ElideRight
            }
        }
    }

    // Tertiary: where the numbers physically came from and when. Both describe
    // the measurement rather than the place, so they sit together directly under
    // the reading they qualify, at the same rank as the field names below.
    RowLayout {
        Layout.fillWidth: true
        spacing: conditionsView.theme.spaceMd

        Label {
            id: stationLabel
            text: conditionsView.weatherController.stationName.toUpperCase()
            visible: stationLabel.text.length > 0
            font.pointSize: conditionsView.theme.fontCaption
            font.bold: true
            font.letterSpacing: conditionsView.theme.capsTracking
            color: conditionsView.theme.text
            opacity: conditionsView.theme.emphasisLow
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Label {
            id: observedLabel
            text: conditionsView.weatherController.observedLabel.toUpperCase()
            visible: observedLabel.text.length > 0
            font.pointSize: conditionsView.theme.fontCaption
            font.bold: true
            font.letterSpacing: conditionsView.theme.capsTracking
            font.features: ({ tnum: 1 })
            color: conditionsView.theme.text
            opacity: conditionsView.theme.emphasisLow
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: metricsRow.implicitHeight + conditionsView.theme.spaceMd * 2
        color: conditionsView.theme.panel
        radius: conditionsView.theme.radiusMd

        RowLayout {
            id: metricsRow
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: conditionsView.theme.spaceLg
            anchors.rightMargin: conditionsView.theme.spaceLg
            spacing: conditionsView.theme.spaceMd

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
