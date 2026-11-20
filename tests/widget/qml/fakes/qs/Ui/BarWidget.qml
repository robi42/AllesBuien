import QtQuick
import Script

// Stands in for the base type Omarchy's bar gives every widget: the two
// properties the bar sets, and a relay. This relay is the check's own. It
// calls the named function on every copy the fake desktop holds, which is all
// the widget asks of the real one; how the real bar finds a widget's copies on
// other monitors is Omarchy's, and only a real bar shows it.
Item {
  property QtObject bar: null
  property string moduleName: ""

  function broadcast(method) {
    Desktop.copies.forEach(function (copy) { copy[method]() })
  }
}
