import QtQuick

// The one place a themed colour is resolved. Rust decides whether an Omarchy
// theme is readable and exposes that as `themeAvailable`; when it is false every
// role falls back to the matching SystemPalette role. Pairing each theme key with
// its fallback exactly once here is what keeps the guard out of every use site —
// and what keeps a single SystemPalette in the tree instead of one per component.
//
// It also carries the geometry and type scales, for the same reason: eight files
// that each invent their own 14, 20 or 13 do not share a rhythm. Everything a
// second file could plausibly need is a token here; a size used in exactly one
// place stays a named readonly property in that file.
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

    // Spacing. One 4 px scale, five steps, no in-between values: every margin,
    // gap and pad in the tree resolves to one of these, which is what makes the
    // three panels read as one vertical rhythm rather than three near misses.
    readonly property int spaceXs: 4
    readonly property int spaceSm: 8
    readonly property int spaceMd: 12
    readonly property int spaceLg: 16
    readonly property int spaceXl: 24

    // Corner radii, one step per surface class: swatch, inset panel, control,
    // outer card.
    readonly property int radiusSm: 4
    readonly property int radiusMd: 8
    readonly property int radiusLg: 12
    readonly property int radiusXl: 16

    // Type scale. Six steps, each with exactly one job — the reading, the frame
    // clock, the condition sentence, a value, a field name, and every uppercase
    // caption. A seventh step would mean two things competing at the same rank.
    readonly property int fontHero: 38
    readonly property int fontLead: 16
    readonly property int fontBody: 13
    readonly property int fontValue: 12
    readonly property int fontLabel: 11
    readonly property int fontCaption: 9
    // Uppercase captions need tracking to stay readable at caption size.
    readonly property real capsTracking: 1.1

    // Emphasis. 0.7 is the floor, not a preference: `text` over `surface`
    // measures 5.2:1 at 0.7 and 4.3:1 at 0.6 on Nord, so anything fainter
    // fails AA for the caption sizes these opacities are used at.
    readonly property real emphasisFull: 1.0
    readonly property real emphasisMid: 0.85
    readonly property real emphasisLow: 0.7
    // Scrims that sit over the radar raster and have to stay readable on it.
    readonly property real scrimOpacity: 0.92

    // Sizes two files have to agree on.
    readonly property int iconHero: 56
    readonly property int iconRow: 34
    readonly property int hairline: 1
}
