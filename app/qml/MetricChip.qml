import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// One labelled measurement: a tracked uppercase field name over its value, the
// pattern every caption in this app follows. Collapses itself when the station
// omitted the value, so an absent reading leaves no stray label behind.
ColumnLayout {
    id: metricChip

    required property Theme theme
    property string label: ""
    property string value: ""

    spacing: 0
    visible: metricChip.value.length > 0

    Label {
        text: metricChip.label
        font.pointSize: metricChip.theme.fontCaption
        font.bold: true
        font.letterSpacing: metricChip.theme.capsTracking
        color: metricChip.theme.text
        opacity: metricChip.theme.emphasisLow
        Layout.fillWidth: true
        elide: Text.ElideRight
    }

    Label {
        text: metricChip.value
        font.pointSize: metricChip.theme.fontValue
        font.bold: true
        // Rewritten on every refresh, and three of these sit in one row; without
        // tabular figures each one's width moves and the row reflows around it.
        font.features: ({ tnum: 1 })
        color: metricChip.theme.text
        Layout.fillWidth: true
        elide: Text.ElideRight
    }
}
