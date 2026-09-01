import QtQuick

// The one place a themed colour is resolved. Rust decides whether an Omarchy
// theme is readable and exposes that as `themeAvailable`; when it is false every
// role falls back to the matching SystemPalette role. Pairing each theme key with
// its fallback exactly once here is what keeps the guard out of every use site —
// and what keeps a single SystemPalette in the tree instead of one per component.
QtObject {
    id: theme

    required property var themeController

    property SystemPalette fallback: SystemPalette {}
    readonly property bool available: theme.themeController.themeAvailable

    // Surfaces.
    readonly property color background: theme.available ? theme.themeController.themeBackground : theme.fallback.window
    readonly property color surface: theme.available ? theme.themeController.themeLighterBackground : theme.fallback.alternateBase
    readonly property color panel: theme.available ? theme.themeController.themeBackground : theme.fallback.base

    // Text.
    readonly property color text: theme.available ? theme.themeController.themeForeground : theme.fallback.text
    readonly property color accent: theme.available ? theme.themeController.themeAccent : theme.fallback.highlight
    readonly property color error: theme.available ? theme.themeController.themeAccent : "red"

    // Separators. Both are the theme's selection colour; only the fallback differs,
    // because a row rule and a panel rule sit on different surfaces.
    readonly property color selection: theme.available ? theme.themeController.themeSelection : theme.fallback.mid
    readonly property color rowSeparator: theme.available ? theme.themeController.themeSelection : theme.fallback.alternateBase

    // Radar transport controls.
    readonly property color buttonFill: theme.available ? theme.themeController.themeMuted : theme.fallback.button
    readonly property color buttonText: theme.available ? theme.themeController.themeBackground : theme.fallback.buttonText
    readonly property color sliderTrack: theme.available ? theme.themeController.themeBackground : theme.fallback.midlight
}
