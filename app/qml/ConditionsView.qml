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

    spacing: conditionsView.theme.spaceMd

    // The header is two columns, and the split is the question and its answer:
    // on the left the place the app was asked about and where that place came
    // from, in the upper right the instrument the numbers came off and when it
    // read them.
    //
    // Tertiary, all four of them. The name was a 14 pt bold headline and the
    // second-largest thing on screen for a value the viewer chose and already
    // knows, and 11 pt tracked caps is what that argument reduced it to. It has
    // since gone back up twice, and both steps are one correction rather than a
    // drift: an anchor that is merely first among four captions is not an
    // anchor, and what put three captions on this row is the corner filling up.
    //
    // 13 is `theme.fontBody`, and that matters past the number. The
    // intermediate 12 matched `theme.fontValue`, but that step's job is a
    // measured value - the wind and temperature figures - and a place name is
    // no measurement, so taking its size would have conflated two kinds of
    // thing. Hence the local property, declared here and defended against
    // that file's rule that a seventh step would mean two things competing at
    // the same rank. 13 needs no such defence. It is an existing step, shared
    // with the condition sentence, and the two cannot compete because they
    // differ in weight, case, tracking, column and opacity - sharing a size
    // step is what `fontCaption` already does four times over on this very
    // row. So the local property is gone with the point it was invented for,
    // the size is a token again, and the reading keeps its rank at 38 pt. Two
    // scale steps separate them now, `fontLead` the only rung between - one
    // fewer than at the abandoned 12 - but rank never rode the step count
    // alone: 38 against 13 is nearly three to one before weight and position
    // say a word.
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
    // and that is what pays for the extra point. It used to span the card's
    // full width on a row of its own, between the reading and the metrics panel
    // - which put the two captions that qualify the reading after it, and spent
    // a row of card height doing so. In the corner they spend none: two 9 pt
    // lines stack shorter than the name over its provenance, so the header's
    // height is still the left column's height and the card is one row shorter
    // than it was.
    //
    // That corner now says `SCHIPHOL` and `@ 14:10`, two words fewer than the
    // `STATION SCHIPHOL` / `OBSERVED 14:10` it arrived as, and the reversal is
    // a change of premise rather than a change of mind. Both qualifiers were
    // argued for, and `core/src/station.rs` still carries the original argument
    // beside the reason it lapsed. It was made against the full-width row above:
    // level with nothing, at the width of a headline, a bare `SCHIPHOL` really
    // would have read as a second place competing with `AMSTERDAM`, and a bare
    // `14:10` as an unlabelled clock. None of that survives the move. Position
    // says caption rather than headline, because that is what this window puts
    // in a corner; the pairing says the two answer one question between them, so
    // a place stacked on a time can only be an instrument and when it read; and
    // the shared right edge ties both to the location opposite rather than to
    // the reading below. Three signals already carry what the words carried, and
    // a word that restates three signals only spends width - width this corner
    // has to find for `HOEK VAN HOLLAND`, a real station name, inside half a
    // header. `OBSERVED` was also the second one on screen: `RadarTransport.qml`
    // spends that exact word distinguishing past frames from forecast ones,
    // which is a distinction this timestamp is not making, and a duplicate
    // invites it to be read as one. That label keeps the word; this one has no
    // claim on it.
    //
    // What the corner does keep is the RELATION, and it now costs one character
    // instead of eight. Dropping `OBSERVED` bought width and took the "at" with
    // it: `SCHIPHOL` over a bare `12:30` names an instrument and a number and
    // leaves the reader to supply the preposition between them. The pairing
    // argument above only ever gets as far as "these two answer one question
    // together", which is exactly why the word was defensible to begin with.
    // `@ ` finishes the sentence the station starts, in the one glyph a caption
    // row can afford, and it is the same move `location_notice.rs` makes on the
    // opposite column - `VIA OMARCHY` rather than `OMARCHY SETTING` - so both
    // halves of this header now read as predicates of the name above them.
    //
    // It is prefixed here rather than in `conditions_view.rs` because it is
    // typography, not data. The controller keeps carrying a plain `hh:mm`,
    // which is what `app/tests/weather_controller.rs` asserts and what a
    // notification title would want; the decorated string exists only where it
    // is drawn.
    //
    // And it is a guarded ternary rather than a plain `"@ " + text`, because
    // the guard is the only thing that lets this corner still disappear.
    // `visible` binds this label's OWN text length, so an unconditional prefix
    // would pin that length at two, leave an `@` with nothing behind it on
    // screen for a feed that reports no time, and take `headerCorner` down with
    // it - the collapse `app/tests/conditions_header_corner.rs` exists to hold.
    // The uppercasing stays for the reason it was ever there: `datetime.rs`'s
    // fallback passes through the first five characters of a feed value it
    // could not parse, so this label is not guaranteed to be digits.
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
    // any pixel reach the colour the binding asks for, and that label's stroke
    // sat under the 4.5:1 AA asks of text this size while being *specified* at
    // 4.57:1. The control was beside it all along - `observedLabel` differed
    // from it in weight and in nothing else - so the lever is weight, not
    // opacity. `emphasisLow` is the floor Theme.qml documents and there was
    // never any room under it, and reaching up instead would have bought the
    // contrast by spending the name's rank, which is exactly what a header
    // carrying three captions cannot afford.
    //
    // Measured on the shipped build at display scale 1.25, from rendered pixels
    // over the card, as max / brightest decile with the stroke's pixel count and
    // rendered width beside them. Both states were captured from the same
    // compositor at the same window origin, because a capture taken seconds
    // after a raise reads every foreground about 2% dark while the background is
    // unmoved - which is enough to invent a difference this change did not make.
    // The control is the hero glyph, whose colour no part of this changed: it
    // reads (213,219,230) in both.
    //
    //                       contrast          ink    width
    //   AMSTERDAM           7.24:1 / 7.24:1   1986   144 px      before
    //   AMSTERDAM           7.24:1 / 7.24:1   2229   153 px      after
    //   VIA OMARCHY         4.47:1 / 4.43:1   1252   126 px
    //   STATION SCHIPHOL    4.47:1 / 4.43:1   1708   168 px      before
    //   SCHIPHOL            4.47:1 / 4.43:1    930    88 px      after
    //   OBSERVED hh:mm      4.47:1 / 4.45:1   1570   147 px      before
    //   hh:mm               4.47:1 / 4.44:1    517    49 px      after
    //   @ hh:mm             4.52:1 / 4.45:1    653    71 px      with the `@`
    //
    // Only the timestamp's ink is not reproducible to the pixel across runs -
    // it is live data, and 490 to 653 is the spread over which digits the clock
    // happened to be showing and whether the `@` is in front of them. Its width
    // is fixed per state: the label asks for tabular figures for exactly this
    // reason, and the digit columns land on the same pixels either way - `12:30`
    // occupied x 1126..1174 before the glyph and `12:40` occupies x 1126..1174
    // after it, with the `@` added at x 1104..1119 and a six-pixel space behind
    // it against the one pixel that separates two digits.
    //
    // So the glyph costs 22 device px on this line and NOTHING on the corner,
    // which was the thing worth checking. A `ColumnLayout` is as wide as its
    // widest child, `SCHIPHOL` is 88 px against the timestamp's 71, and every
    // other label re-measured to the pixel across that pair - the name's ink
    // bbox at (740,54,892,71), its provenance at (740,79,864,91), the station
    // at (1087,53,1174,65), unmoved - with the hero glyph reading (213,219,230)
    // in both frames as the control that says the two captures are comparable
    // at all. The corner only widens for a station short enough to lose to
    // `@ hh:mm`, which is roughly six characters; the half-header cap measured
    // 218 px against a 435 px header row, so there is no arrangement of these
    // two lines that crowds anything.
    //
    // That `@ hh:mm` row comes from its own capture pair, taken the same way on
    // a later day, which is why the widths above it reproduce and the ink counts
    // do not: the name re-measured 153 px at 2191 ink against 2229, the station
    // 88 px at 887 against 930, the bare timestamp 49 px, and the provenance
    // 125 px against 126. Widths are the figures to compare between the two
    // pairs; ink swings a few percent with the device-row phase a window
    // happens to land on, which is why the origin is pinned within a pair.
    //
    // Read the contrast column first, because it is the one that did not move:
    // a point of type size buys ink and width, never contrast, so 13 pt is not
    // what makes the name legible and never was. What it buys is 12% more ink
    // over 9 px more width, and the corner giving back 80 px of the 168 it held
    // is what that width is taken from rather than from the name.
    //
    // The name holds 1.62x the contrast of the loudest caption on the row and
    // 2.18x its ink per character. Per character is the honest density figure:
    // raw ink flattered `STATION SCHIPHOL` at sixteen characters against the
    // name's nine, and now flatters nothing, because the longest caption left is
    // `VIA OMARCHY` at eleven. Every rendered figure sits a few percent under the
    // colour it was specified at, because antialiasing never quite fills a stem -
    // which is why the decile is quoted beside the max and why neither is the
    // criterion. WCAG 1.4.3 is assessed on the specified pair: 7.45:1 for the
    // name at `emphasisFull`, 4.57:1 for every caption at `emphasisLow`, a
    // specified margin of 1.63x.
    RowLayout {
        objectName: "headerRow"
        Layout.fillWidth: true
        spacing: conditionsView.theme.spaceMd

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Label {
                text: conditionsView.weatherController.locationName.toUpperCase()
                font.pointSize: conditionsView.theme.fontBody
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
        // stated reason. `headerRow` and `headerCorner` carry objectNames so
        // `app/tests/conditions_header_corner.rs` can read that collapse off the
        // rendered tree - a binding that says `visible: false` proves nothing
        // about the gap in front of it, which is a layout outcome.
        //
        // Half the header is the cap because neither side may starve the other
        // by construction. Nothing on this row is fixed-width - the station is
        // whatever Buienradar named the nearest instrument, and `HOEK VAN
        // HOLLAND` is a real one - so without a cap a long station name would
        // take its width out of the name, which is the one thing here that is
        // not allowed to shrink.
        ColumnLayout {
            objectName: "headerCorner"
            visible: stationLabel.visible || observedLabel.visible
            Layout.alignment: Qt.AlignRight | Qt.AlignTop
            Layout.maximumWidth: conditionsView.width / 2
            spacing: 0

            Label {
                id: stationLabel
                objectName: "stationLabel"
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
                objectName: "observedLabel"
                text: conditionsView.weatherController.observedLabel.length > 0
                    ? "@ " + conditionsView.weatherController.observedLabel.toUpperCase()
                    : ""
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

            // Primary — and the only label in the window whose placeholder has
            // to be chosen for its size rather than for its meaning. `elide`
            // below is right for a real reading that outgrows the column, and
            // wrong for a word standing in for one: at 38 pt `Unavailable` ran
            // past the end and cut to `Unavaila…`, which does not read as a
            // missing value at all — it reads as a value the app is too small
            // to print. Rust sends `status.NO_READING` here instead, and the
            // choice is made there rather than by a ternary on the string this
            // label happens to be carrying. `objectName` lets
            // `app/tests/conditions_failure_line.rs` read the elision off the
            // rendered tree, which is a layout outcome and not a binding.
            Label {
                id: heroReading
                objectName: "heroReading"
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

    // The single owner of a weather-feed failure. `ForecastView` used to bind
    // the same `status` and print the same sentence a second time below this
    // one; its copy is gone, and this line now scopes the whole right column,
    // including the empty list under it. `RadarPanel` keeps its own status
    // because `radarController.status` is a different fetch that fails
    // separately — two sentences there would be two faults, not one repeated.
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
