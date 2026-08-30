# QML Radar View & Layout Architecture (Task 17)
- **Main Layout**: `Main.qml` uses a `RowLayout`. Left side is the `RadarView` (expanding), right side is a 320px wide `ColumnLayout` containing placeholders for `conditionsView` and `forecastView`.
- **Radar Composite Strategy**: `basemap` (1058x915) is rendered using `Image.Stretch` and locked to the correct aspect ratio (1058/915). The rain `overlay` (which has the exact same dimensions) is a child of the `basemap` with `anchors.fill: parent` and `fillMode: Image.Stretch`. This ensures both layers scale perfectly together with 100% deterministic geometry alignment at any window size.
- **Playback Controls**: Scrubbing stops playback and updates `currentFrameIndex`, `currentFrameUrl`, and `currentFrameLabel` immediately via auto-generated CXX-Qt property setters to ensure UI remains responsive when dragged.
- **Theming**: Fallback gracefully utilizes `SystemPalette` when `themeController.themeAvailable` is false.
- **Dependency Flow**: Placeholders `conditionsView` and `forecastView` are ready for Task 18.
