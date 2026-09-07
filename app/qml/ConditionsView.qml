import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Three ranks, and only three. Primary is the reading — it is the largest thing
// in the window and nothing else comes near it. Secondary is what the reading
// means: the condition sentence and the three measured values. Tertiary is every
// name and caption, uppercase and tracked at caption sizes, so they read as
// field names rather than as competing headlines.
ColumnLayout {
    id: conditionsView

    required property var weatherController
    required property Theme theme

    // One point above the field-name step, and named here rather than added to
    // Theme.qml's six-step scale because that file states the rule: everything
    // a second file could plausibly need is a token there, and a size used in
    // exactly one place stays a named property where it is used. Nothing else
    // in this tree is a panel header, so there is nothing for a seventh step to
    // share. Why 12 and not 11 is argued with the header below.
    readonly property int locationNameSize: conditionsView.theme.fontLabel + 1

    spacing: conditionsView.theme.spaceMd

    // The header is two columns, and the split is the question and its answer:
    // on the left the place the app was asked about and where that place came
    // from, in the upper right the instrument the numbers came off and when it
    // read them.
    //
    // Tertiary, all four of them. The name was a 14 pt bold headline and the
    // second-largest thing on screen for a value the viewer chose and already
    // knows, and 11 pt tracked caps is what that argument reduced it to. It
    // goes back up here, by one point and only one, because the argument's
    // premise changed rather than its reasoning: the name now shares the header
    // with three caption-weight items instead of one, and an anchor that is
    // merely first among four captions is not an anchor. A point is a third of
    // what the earlier reduction took away, it leaves the reading three scale
    // steps clear at 38 pt, and it is checked rather than assumed - the
    // measured margins are quoted at the bottom of this comment.
    //
    // Under the name rides the provenance, because the name on its own could
    // not tell a chosen place from a fallen-back one: Amsterdam-West rendered
    // identically whether the viewer had picked it or never configured
    // anything. Rust decides the word; this only places it. It used to ride the
    // name's row flush right, 131 px clear of the end of `AMSTERDAM` and 266 px
    // from where the eye starts the line; on its own line under the name that
    // left-edge offset is 0, the two share a reading edge, and the caption is
    // found on the way down to the reading rather than sought out. It reads as
    // the name's predicate, which is why the wording in `location_notice.rs` is
    // prepositional: directly under a place name `OMARCHY SETTING` is a second
    // noun and leaves the relation to be guessed, where `VIA OMARCHY` finishes
    // the sentence the name starts.
    //
    // The corner that move vacated is where the measurement source now sits,
    // and that is what pays for the extra point. `STATION SCHIPHOL` and
    // `OBSERVED 14:10` used to span the card's full width on a row of their
    // own, between the reading and the metrics panel - which put the two
    // captions that qualify the reading after it, and spent a row of card
    // height doing so. In the corner they spend none: two 9 pt lines stack
    // shorter than the name over its provenance, so the header's height is
    // still the left column's height and the card is one row shorter than it
    // was.
    //
    // They stay a pair, and they stack. A pair because they answer one
    // question between them - which instrument, at what time - and splitting
    // them would leave the other caption in some corner with no better claim to
    // it. Stacked because the card's full width is what used to hold them
    // apart; in half a header, inline, only a separator glyph would, and this
    // file has no separator glyph to spend. Stacked they get a shared right
    // reading edge that mirrors the left column's shared left one, so the
    // header reads as two blocks rather than as four loose captions.
    //
    // All four are bold, the station included - and the station is the one that
    // was written out as regular on purpose, which was a measured defect rather
    // than a preference. Opacity is a specification, not an outcome: at 9 pt a
    // regular stem is thinner than a device pixel, so antialiasing never lets
    // any pixel reach the colour the binding asks for. The station's brightest
    // rendered pixel measured 4.05:1 against the card and its brightest decile
    // 3.29:1, under the 4.5:1 AA asks of text this size, while the colour it
    // was *specified* at is 4.57:1. The control was already beside it:
    // `observedLabel` differed from it in weight and in nothing else, and
    // measured 4.57:1 max and 4.44:1 decile. So the lever is weight, not
    // opacity. `emphasisLow` is the floor Theme.qml documents and there was
    // never any room under it, and reaching up instead would have bought the
    // contrast by spending the name's rank - which is exactly what a header
    // carrying three captions cannot afford.
    //
    // Measured on the shipped build at display scale 1.25, from rendered pixels
    // over the card, as max / brightest decile with the stroke's pixel count
    // beside it:
    //
    //   AMSTERDAM         7.31:1 / 7.24:1   ink 1974
    //   VIA OMARCHY       4.47:1 / 4.43:1   ink 1226
    //   STATION SCHIPHOL  4.48:1 / 4.43:1   ink 1673   (was 4.05:1 / 3.29:1)
    //   OBSERVED 14:10    4.57:1 / 4.46:1   ink 1479
    //
    // The name holds 1.63x the contrast of the caption beneath it and 1.61x its
    // ink - both a shade wider than before the corner filled up - and 2.1x the
    // ink per character of the longest caption on the row. Per character is the
    // honest density figure here: raw ink flatters `STATION SCHIPHOL` only
    // because it is sixteen characters against the name's nine.
    //
    // The station's remaining 0.07 to 4.50:1 is device-row phase rather than
    // anything this file specifies. The same build captured at four window
    // origins one logical pixel apart reads 4.57:1, 4.56:1, 4.56:1 and 4.57:1
    // for that label, and every rendered figure above sits about 3% under the
    // colour it was specified at because antialiasing never quite fills a stem
    // - the name is 7.24:1 rendered against 7.45:1 specified by the same tax.
    // That is why the decile is quoted beside the max and why neither is the
    // criterion: WCAG 1.4.3 is assessed on the specified pair, and this one is
    // 4.57:1.
    RowLayout {
        Layout.fillWidth: true
        spacing: conditionsView.theme.spaceMd

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Label {
                text: conditionsView.weatherController.locationName.toUpperCase()
                font.pointSize: conditionsView.locationNameSize
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

        // Collapsed as a block, not just per label: with both labels hidden the
        // two-item stack still has a row gap in front of it, and a feed that
        // names no station would leave the name ending 12 px early for no
        // stated reason.
        //
        // Half the header is the cap because neither side may starve the other
        // by construction. Nothing on this row is fixed-width - the station is
        // whatever Buienradar named the nearest instrument, and `STATION HOEK
        // VAN HOLLAND` is a real one - so without a cap a long station name
        // would take its width out of the name, which is the one thing here
        // that is not allowed to shrink.
        ColumnLayout {
            visible: stationLabel.visible || observedLabel.visible
            Layout.alignment: Qt.AlignRight | Qt.AlignTop
            Layout.maximumWidth: conditionsView.width / 2
            spacing: 0

            Label {
                id: stationLabel
                text: conditionsView.weatherController.stationName.toUpperCase()
                visible: stationLabel.text.length > 0
                font.pointSize: conditionsView.theme.fontCaption
                font.bold: true
                font.letterSpacing: conditionsView.theme.capsTracking
                color: conditionsView.theme.text
                opacity: conditionsView.theme.emphasisLow
                horizontalAlignment: Text.AlignRight
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
                horizontalAlignment: Text.AlignRight
                Layout.fillWidth: true
                elide: Text.ElideRight
            }
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
