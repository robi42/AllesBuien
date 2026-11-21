import QtQuick

// The one place a themed colour is resolved. Rust decides whether an Omarchy
// theme is readable and exposes that as `themeAvailable`; when it is false every
// role falls back to the matching SystemPalette role. Pairing each theme key with
// its fallback exactly once here is what keeps the guard out of every use site —
// and what keeps a single SystemPalette in the tree instead of one per component.
//
// It also carries the geometry and type scales, for the same reason: files that
// each invent their own 14, 20 or 13 do not share a rhythm. Everything a
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
    // SystemPalette has no error role, so this fallback is the tree's one literal
    // colour; bin/check-fast's colour gate allows it here and nowhere else.
    readonly property color error: theme.available ? theme.themeController.themeAccent : "red"

    // The theme's selection colour, in two roles with different fallbacks: the
    // play button's pressed fill, and the rule between forecast rows.
    readonly property color selection: theme.available ? theme.themeController.themeSelection : theme.fallback.mid
    readonly property color rowSeparator: theme.available ? theme.themeController.themeSelection : theme.fallback.alternateBase

    // Radar transport controls.
    readonly property color buttonFill: theme.available ? theme.themeController.themeMuted : theme.fallback.button
    readonly property color buttonText: theme.available ? theme.themeController.themeBackground : theme.fallback.buttonText
    readonly property color sliderTrack: theme.available ? theme.themeController.themeBackground : theme.fallback.midlight
    // The keyboard-focus ring. The foreground rather than the accent: the
    // slider handle is already accent-filled, so an accent ring would vanish on
    // exactly the control it has to mark, while the foreground is the one
    // colour every theme promises contrasts with each surface it sits on here.
    readonly property color focusRing: theme.text

    // Spacing. One 4 px scale, five steps, no in-between values: every margin,
    // gap and pad resolves to one of these, which is what makes the three
    // panels read as one vertical rhythm rather than three near misses. The
    // legend's 2 px between a swatch and its caption is the one exception.
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

    // Type scale. Six steps, each named for its main job — the reading, the frame
    // clock, the condition sentence, a value, a field name, and the small
    // captions. A seventh step would mean two things competing at the same rank.
    readonly property int fontHero: 38
    readonly property int fontLead: 16
    readonly property int fontBody: 13
    readonly property int fontValue: 12
    readonly property int fontLabel: 11
    readonly property int fontCaption: 9
    // Uppercase captions need tracking to stay readable at caption size.
    readonly property real capsTracking: 1.1

    // Emphasis. 0.7 is the floor, not a preference: `text` over `surface`
    // measures 4.6:1 at 0.7 and 3.8:1 at 0.6 on Nord, so anything fainter
    // fails AA for the caption sizes these opacities are used at.
    readonly property real emphasisFull: 1.0
    readonly property real emphasisMid: 0.85
    readonly property real emphasisLow: 0.7
    // Scrims that sit over the radar raster and have to stay readable on it.
    readonly property real scrimOpacity: 0.92

    // Omarchy's text size, as the factor the window zooms its whole content
    // by: 1 at the default 12 px. The bridge counts it in twelve-thousandths,
    // so that base-size / 12 is a whole number for every whole base size and
    // no float crosses into Rust; 12000 is `ZOOM_UNIT` in
    // core/src/window_fit.rs, and the text-size GUI test pins the two together.
    readonly property real zoom: theme.themeController.zoomParts / 12000

    // Sizes two files have to agree on.
    readonly property int iconHero: 56
    readonly property int iconRow: 34
    readonly property int hairline: 1
}
