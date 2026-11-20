pragma Singleton
import QtQml

// The machine the widget runs on, as the check scripts it: what a probe
// answers from now on, and the widget's live copies, one per monitor.
QtObject {
  property int answer: 0
  property var copies: []
}
