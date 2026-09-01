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
    visible: true
    title: "AllesBuien"
    color: appTheme.background

    Theme {
        id: appTheme
        themeController: root.themeSource
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

            // 380, down from 420. The forecast row carries four short columns
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

    Timer {
        interval: 50
        running: Qt.platform.pluginName === "offscreen"
        onTriggered: Qt.quit()
    }
}
