import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// One labelled measurement. Collapses itself when the station omitted the
// value, so an absent reading leaves no stray label behind.
ColumnLayout {
    id: metricChip

    required property Theme theme
    property string label: ""
    property string value: ""

    spacing: 1
    visible: metricChip.value.length > 0

    Label {
        text: metricChip.label
        font.pointSize: 9
        font.bold: true
        font.letterSpacing: 0.8
        color: metricChip.theme.text
        opacity: 0.7
        Layout.fillWidth: true
        elide: Text.ElideRight
    }

    Label {
        text: metricChip.value
        font.pointSize: 12
        font.bold: true
        color: metricChip.theme.text
        Layout.fillWidth: true
        elide: Text.ElideRight
    }
}
