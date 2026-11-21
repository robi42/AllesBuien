import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root

    // Rust hands the three controllers to QML as engine context properties
    // (`rootContext()->setContextProperty(...)` in app/src/*_controller.cpp).
    // A context property has no QML declaration for qmllint to resolve, so these
    // three reads are the only unqualified access left in the tree. Every
    // component below receives them as declared `required property` inputs
    // instead of reaching back up into root scope.
    readonly property var weatherSource: weatherController // qmllint disable unqualified
    readonly property var radarSource: radarController // qmllint disable unqualified
    readonly property var themeSource: themeController // qmllint disable unqualified

    // The radar frames and the rain graph are separate feeds whose only shared
    // basis is the viewer's wall clock, and Rust maps each onto the other: the
    // frame on screen places the graph's cursor, and the graph's last bar is
    // where the radar's animation ends. These two pairs are the only place in
    // the tree where the two controllers meet. Each bridge keeps the last value
    // the other handed it, so it does not matter which feed answers first, and
    // both values start empty and change only once the event loop runs, after
    // this file has loaded, so each handler below sees every change.
    readonly property string radarFrameLabel: root.radarSource.currentFrameLabel
    onRadarFrameLabelChanged: root.weatherSource.trackRadarFrame(root.radarFrameLabel)
    readonly property string rainEndLabel: root.weatherSource.rainEndLabel
    onRainEndLabelChanged: root.radarSource.trackRainEnd(root.rainEndLabel)

    objectName: "rootWindow"
    // The window's sizes follow Omarchy's text size, so they come from the
    // theme bridge, which zooms the design sizes and caps the zoom by the
    // screen. The design sizes themselves, 852×672 at the least and 980×760 to
    // open at, and the reasoning behind each, are `WINDOW` in
    // app/src/theme_controller.rs.
    width: root.themeSource.openingWidth
    height: root.themeSource.openingHeight
    minimumWidth: root.themeSource.minimumWidth
    minimumHeight: root.themeSource.minimumHeight
    visible: true
    title: "AllesBuien"
    color: appTheme.background

    Theme {
        id: appTheme
        themeController: root.themeSource
    }

    // A floating single-window utility that is opened from a keybind gets
    // closed from the keyboard too. Escape is the only key this file binds:
    // the radar's playback keys are window-context Shortcuts in
    // RadarTransport.qml, beside the two controls they drive, and the tab
    // order is that file's item order.
    Shortcut {
        sequences: ["Escape"]
        onActivated: Qt.quit()
    }

    // The window's two hand-offs to the theme bridge, which only the window
    // can make. The screen it is on caps the zoom and fits the opening size;
    // the first frame starts the end of the launch, after which the opening
    // size stays put, and once the launch is over the window stops listening.
    ScreenReport {
        objectName: "screenReport"
        onReported: (screenWidth, screenHeight, workWidth, workHeight) => root.themeSource.trackScreen(screenWidth, screenHeight, workWidth, workHeight)
    }

    Connections {
        target: root
        enabled: root.themeSource.launching

        function onFrameSwapped() {
            root.themeSource.trackFrame();
        }
    }

    // Omarchy's text size zooms the whole content, so the layout stays the one
    // that was tested at 12 px. The wrapper is laid out at the window's size
    // divided by the zoom and scaled back up from its top-left corner, so it
    // always fills the window exactly.
    Item {
        id: zoomRoot
        objectName: "zoomRoot"
        width: parent.width / appTheme.zoom
        height: parent.height / appTheme.zoom
        scale: appTheme.zoom
        transformOrigin: Item.TopLeft

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: appTheme.spaceXl
            spacing: appTheme.spaceLg

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: appTheme.spaceXl

                RadarPanel {
                    objectName: "radarView"
                    radarController: root.radarSource
                    theme: appTheme
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumWidth: 400
                }

                // 380, down from 420. The forecast row carries three short columns
                // and nothing else; at 420 they could not be spread without leaving
                // a hole, and the 40 px goes to the radar, which is the one thing
                // here that gets better the larger it is.
                Rectangle {
                    Layout.preferredWidth: 380
                    Layout.fillHeight: true
                    color: appTheme.surface
                    radius: appTheme.radiusXl

                    // No standalone rule between the conditions block and the list:
                    // the metrics panel already ends the block with a filled edge,
                    // and the list draws its own row rules. The rule that used to
                    // sit here cost 33 px of column and separated nothing.
                    ColumnLayout {
                        id: rightCol
                        anchors.fill: parent
                        anchors.margins: appTheme.spaceLg
                        spacing: appTheme.spaceMd

                        ConditionsView {
                            objectName: "conditionsView"
                            weatherController: root.weatherSource
                            theme: appTheme
                            Layout.fillWidth: true
                        }

                        // fillHeight without a preferred height: the list takes the
                        // column's whole remainder and divides it across its rows,
                        // so slack becomes row rhythm instead of a void above the
                        // attribution. Binding preferredHeight to contentHeight
                        // while the rows size themselves from the view would be a
                        // loop, which is why that binding is gone.
                        ForecastView {
                            objectName: "forecastView"
                            weatherController: root.weatherSource
                            theme: appTheme
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                        }

                        // Buienradar's free-data terms ask for three things: the use
                        // stays non-commercial, the words `Buienradar.nl` are
                        // visible, and they link to the root domain. `Bron:` was
                        // ours, not theirs — a Dutch source label on an otherwise
                        // English surface, naming what the domain already names.
                        //
                        // The URL is a property rather than a literal inside the
                        // handler so the GUI test can read the licence's actual
                        // invariant off the rendered object — visible domain text
                        // plus a root-domain link — instead of pinning a sentence.
                        Label {
                            id: attribution
                            objectName: "attribution"

                            readonly property string link: "https://www.buienradar.nl"

                            text: "Buienradar.nl"
                            font.pointSize: appTheme.fontCaption
                            font.bold: true
                            font.underline: attributionHover.hovered
                            color: appTheme.text
                            opacity: attributionHover.hovered ? appTheme.emphasisFull : appTheme.emphasisLow
                            Layout.alignment: Qt.AlignRight

                            HoverHandler {
                                id: attributionHover
                                cursorShape: Qt.PointingHandCursor
                            }

                            TapHandler {
                                onTapped: Qt.openUrlExternally(attribution.link)
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 132
                color: appTheme.surface
                radius: appTheme.radiusXl

                RainView {
                    weatherController: root.weatherSource
                    theme: appTheme
                    anchors.fill: parent
                    anchors.margins: appTheme.spaceLg
                }
            }
        }
    }

    Timer {
        interval: root.weatherSource.refreshIntervalMs
        repeat: true
        running: true
        onTriggered: root.weatherSource.refresh()
    }

    Timer {
        interval: root.radarSource.refreshIntervalMs
        repeat: true
        running: true
        onTriggered: root.radarSource.refresh()
    }

    Timer {
        interval: root.radarSource.playbackIntervalMs
        repeat: true
        running: root.radarSource.playing
        onTriggered: root.radarSource.advanceFrame()
    }

    Timer {
        interval: root.themeSource.pollIntervalMs
        repeat: true
        running: true
        onTriggered: root.themeSource.poll()
    }

    // Omarchy's weather.json is a desktop-wide setting that Omarchy's own
    // weather pill follows live, and so does this window. Both bridges
    // re-ask on the same tick and each decides for itself whether the answer
    // moved; neither is told the location by the other, so the order of these
    // two calls carries no meaning. Polled rather than watched for the reason
    // the theme is: --clear removes the file and a later --set writes a new
    // one, which an inode-bound watcher stops hearing about after the first
    // removal.
    Timer {
        interval: root.weatherSource.locationPollIntervalMs
        repeat: true
        running: true
        onTriggered: {
            root.weatherSource.pollLocation();
            root.radarSource.pollLocation();
        }
    }
}
