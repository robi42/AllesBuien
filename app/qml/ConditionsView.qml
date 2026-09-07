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
    // Under it rides the provenance, because the name on its own could not tell
    // a chosen place from a fallen-back one: Amsterdam-West rendered identically
    // whether the viewer had picked it or never configured anything. Rust
    // decides the word; this only places it.
    //
    // It used to ride the name's row, flush right, at `emphasisLow` and regular
    // weight, and it was hard to find for two measured reasons rather than one.
    //
    // The first is that regular weight cannot cash the opacity it is given. At
    // 9 pt a regular stem is thinner than a device pixel, so antialiasing never
    // lets any pixel reach the colour the binding asks for: the caption's
    // brightest pixel measured 3.98:1 against the card and its brightest decile
    // 3.42:1, under the 4.5:1 AA asks of text this size, while the colour it was
    // *specified* at is 4.57:1. Bold is the whole correction. The proof was
    // already on screen before anything changed - `observedLabel` below is the
    // same 9 pt at the same `emphasisLow`, differing only in weight, and its
    // decile measured 4.46:1 where its regular neighbour `stationLabel` reached
    // only 3.42:1. So the fix is weight, not opacity: `emphasisLow` is the
    // floor Theme.qml documents and there was never any room under it, and
    // going up instead would have bought contrast by spending the name's rank.
    //
    // The second is distance. Flush right, the caption sat 131 px clear of the
    // end of `AMSTERDAM` and 266 px from where the eye starts the line - the
    // one part of the header nothing else asks you to look at. Moving it to its
    // own line directly under the name takes that to a 0 px left-edge offset:
    // the two now share a reading edge, and the caption is found on the way
    // down to the reading rather than sought out.
    //
    // Neither move costs the anchor, which is the constraint this replaced an
    // earlier fix to satisfy. The name keeps 11 pt bold at `emphasisFull` and
    // its decile measures 7.24:1 against the caption's 4.45:1, one rank larger
    // and half again more ink, on the line above. It reads as the header; the
    // caption reads as its predicate - which is why the wording in
    // `location_notice.rs` is prepositional now. Directly under a place name,
    // `OMARCHY SETTING` is a second noun and leaves the relation to be guessed;
    // `VIA OMARCHY` finishes the sentence the name starts.
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 0

        Label {
            text: conditionsView.weatherController.locationName.toUpperCase()
            font.pointSize: conditionsView.theme.fontLabel
            font.bold: true
            font.letterSpacing: conditionsView.theme.capsTracking
            color: conditionsView.theme.text
            opacity: conditionsView.theme.emphasisFull
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Label {
            id: locationSource
            objectName: "locationSource"
            text: conditionsView.weatherController.locationSource.toUpperCase()
            visible: locationSource.text.length > 0
            font.pointSize: conditionsView.theme.fontCaption
            font.bold: true
            font.letterSpacing: conditionsView.theme.capsTracking
            color: conditionsView.theme.text
            opacity: conditionsView.theme.emphasisLow
            Layout.fillWidth: true
            elide: Text.ElideRight
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
