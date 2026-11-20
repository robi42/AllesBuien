import QtQuick

// Stands in for the bar's icon button: the properties the widget binds, and
// the press it handles.
Item {
  property var bar: null
  property string text: ""
  property bool dimmed: false
  property bool active: false
  property string tooltipText: ""
  signal pressed()
}
