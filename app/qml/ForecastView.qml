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

    // No status line here, deliberately, and the empty list is not an
    // unexplained void. This view used to carry its own `forecastError` bound to
    // `weatherController.status` — the same property `ConditionsView` binds —
    // so one failed feed printed one sentence twice, about 190 px apart, and
    // the repetition read as two separate faults rather than as one.
    //
    // Both labels answered for the same fetch, so only one of them could be the
    // owner, and it is the one above: the conditions block is primary, it is
    // where the eye lands first, and its sentence sits at the bottom edge of
    // that block — directly above this list with one `spaceMd` between them.
    // Whatever emptiness follows is already inside the scope of the line that
    // just named the cause, in reading order, which is what the second copy was
    // trying to say and could only say redundantly. Removing it beat adding a
    // second condition, because the duplicate could not be suppressed without
    // this view knowing about a sibling it otherwise has no reason to know.
    //
    // Empty for the other reason — a fetch that succeeded and returned no days,
    // where the status is empty and there is nothing to explain — is unchanged:
    // the old label was already hidden in that state by its own `text.length`
    // guard, so nothing that ever rendered here has been taken away.
    clip: true
    model: forecastView.weatherController
    interactive: false

    delegate: Item {
        id: forecastRow

        required property var weekday
        required property var iconUrl
        required property bool iconLoading
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
                    objectName: "forecastIcon"
                    source: forecastRow.iconUrl
                    slotSize: forecastView.theme.iconRow
                    theme: forecastView.theme
                    loading: forecastRow.iconLoading
                    Layout.preferredWidth: rowIcon.implicitWidth
                    Layout.preferredHeight: rowIcon.implicitHeight
                }

                // A nested layout defaults to fillWidth, which would let this
                // cell swallow the free space and strand the dot beside the
                // icon. Fixed width is the whole point: the chance has to land
                // on the same x in every row.
                //
                // The slack goes in front of the pair, not between it: a
                // right-aligned fill-width Label kept its own leftover inside
                // itself, so "30%" and "70%" pushed the dot a different
                // distance away in every row and the mark stopped reading as
                // attached to its number. A leading spacer takes the whole
                // remainder instead, which leaves one `spaceXs` between dot and
                // digits in all five rows while the right edge stays put. An
                // invisible dot is skipped by RowLayout together with its
                // spacing, so a zero-chance day still measures the same cell.
                RowLayout {
                    Layout.fillWidth: false
                    Layout.preferredWidth: forecastView.rainWidth
                    spacing: forecastView.theme.spaceXs

                    Item {
                        Layout.fillWidth: true
                    }

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
                    }
                }

                Item {
                    Layout.fillWidth: true
                }
            }

            // Two readings, two ranks. The high is what the day is asked for;
            // the overnight low is the context it is read against, and at one
            // shared weight the pair competed and the row had no headline. The
            // low drops to regular weight at `emphasisMid`, the high keeps bold
            // at full strength, and nothing moves: the slack sits in a leading
            // spacer so the column still ends on the same right edge.
            //
            // Tabular figures stay on both, which is what keeps the slash and
            // both readings on the same x down the column, whatever digits a
            // day happens to carry. A nested layout defaults to fillWidth, so
            // this one says otherwise for the same reason the chance cell does.
            RowLayout {
                Layout.fillWidth: false
                Layout.preferredWidth: forecastView.temperatureWidth
                spacing: 0

                Item {
                    Layout.fillWidth: true
                }

                Label {
                    text: forecastRow.minTemperature + "° / "
                    font.pointSize: forecastView.theme.fontValue
                    font.features: ({ tnum: 1 })
                    color: forecastView.theme.text
                    opacity: forecastView.theme.emphasisMid
                }

                Label {
                    text: forecastRow.maxTemperature + "°"
                    font.pointSize: forecastView.theme.fontValue
                    font.bold: true
                    font.features: ({ tnum: 1 })
                    color: forecastView.theme.text
                }
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
}
