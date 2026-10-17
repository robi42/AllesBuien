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
    // basis is the viewer's wall clock. Rust maps one onto the other; this pair
    // is the single point in the tree where the two controllers meet.
    readonly property string radarFrameLabel: root.radarSource.currentFrameLabel
    onRadarFrameLabelChanged: root.weatherSource.trackRadarFrame(root.radarFrameLabel)

    objectName: "rootWindow"
    width: 980
    height: 760
    // The radar's own floor plus the column beside it, the outer margins and
    // the gap between them: 400 + 380 + 24 * 2 + 24. Below that the two panels
    // start taking width from each other, and the first thing to go is the map,
    // which is the one element here that gets worse the smaller it is.
    //
    // The height is measured, not derived: the forecast list clips rather than
    // squashing its rows, so the floor is whatever makes all five of them fit.
    // 640 renders four and slices the fifth, 660 still slices it, 664 lands the
    // last row's icon on the clip edge, 668 is clean. 672 adds one 4 px step as
    // the margin for a themed font with taller metrics, since the window
    // follows whichever theme Omarchy has on.
    minimumWidth: 852
    minimumHeight: 672
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
    // moved; neither is told by the other, so the order of these two calls
    // carries no meaning. Polled rather than watched for the reason the theme
    // is: --clear removes the file and a later --set writes a new one, which an
    // inode-bound watcher stops hearing about after the first removal.
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
