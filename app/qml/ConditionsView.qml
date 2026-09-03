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
    //
    // The provenance rides the same row, one rank quieter, because the name on
    // its own could not tell a chosen place from a fallen-back one: Amsterdam-West
    // rendered identically whether the viewer had picked it or never configured
    // anything. Rust decides the word; this only places it.
    //
    // Quieter had stopped being enough. With a caption rail on this row and
    // another under the reading, the two provenance strings bracketed the
    // header and the name read as one of four peers. The provenance cannot go
    // lower - it already sits at `emphasisLow`, which Theme.qml documents as
    // the measured AA floor for caption sizes - so the name takes the anchor
    // instead, at full opacity. That widens the gap to three simultaneous
    // contrasts, 11pt bold at 1.0 against 9pt regular at 0.7, without either
    // label leaving the tertiary grammar. Baseline alignment does the rest:
    // sitting the caption on the name's baseline reads as an annotation to it,
    // where vertical centring read as a sibling beside it.
    RowLayout {
        Layout.fillWidth: true
        spacing: conditionsView.theme.spaceSm

        Label {
            text: conditionsView.weatherController.locationName.toUpperCase()
            font.pointSize: conditionsView.theme.fontLabel
            font.bold: true
            font.letterSpacing: conditionsView.theme.capsTracking
            color: conditionsView.theme.text
            opacity: conditionsView.theme.emphasisFull
            Layout.alignment: Qt.AlignBaseline
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Label {
            id: locationSource
            objectName: "locationSource"
            text: conditionsView.weatherController.locationSource.toUpperCase()
            visible: locationSource.text.length > 0
            font.pointSize: conditionsView.theme.fontCaption
            font.bold: false
            font.letterSpacing: conditionsView.theme.capsTracking
            color: conditionsView.theme.text
            opacity: conditionsView.theme.emphasisLow
            Layout.alignment: Qt.AlignBaseline
        }
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

        // Written out rather than left to the default: every other caption in
        // this file is bold, and the station is the one that should not be. It
        // is the longest string on the row and it names a place the reading did
        // not come from, so bold made it the loudest tertiary line on screen.
        // Regular weight is the whole correction — the opacity stays at
        // `emphasisLow`, which `Theme.qml` documents as the measured AA floor
        // for caption sizes, so there is nothing below it to reach for.
        Label {
            id: stationLabel
            text: conditionsView.weatherController.stationName.toUpperCase()
            visible: stationLabel.text.length > 0
            font.pointSize: conditionsView.theme.fontCaption
            font.bold: false
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
