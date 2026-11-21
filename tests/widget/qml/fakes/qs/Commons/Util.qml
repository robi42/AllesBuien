pragma Singleton
import QtQml

// The one helper of Omarchy's Util the widget calls. The check reads a command
// only for the verb it carries, so this one quotes nothing.
QtObject {
  function shellQuote(value) {
    return value
  }
}
