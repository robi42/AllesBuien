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
        anchors.margins: 24
        spacing: 16

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 32

            RadarPanel {
                objectName: "radarView"
                radarController: root.radarSource
                theme: appTheme
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 400
            }

            Rectangle {
                Layout.preferredWidth: 420
                Layout.fillHeight: true
                color: appTheme.surface
                radius: 16

                ColumnLayout {
                    id: rightCol
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 16

                    ConditionsView {
                        objectName: "conditionsView"
                        weatherController: root.weatherSource
                        theme: appTheme
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 1
                        color: appTheme.selection
                        opacity: 0.5
                    }

                    ForecastView {
                        id: forecastView
                        objectName: "forecastView"
                        weatherController: root.weatherSource
                        theme: appTheme
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.preferredHeight: forecastView.contentHeight
                    }

                    Label {
                        objectName: "attribution"
                        text: "Bron: Buienradar.nl"
                        font.pointSize: 10
                        font.bold: true
                        font.underline: attributionHover.hovered
                        color: appTheme.text
                        opacity: attributionHover.hovered ? 1.0 : 0.7
                        Layout.alignment: Qt.AlignRight

                        HoverHandler {
                            id: attributionHover
                            cursorShape: Qt.PointingHandCursor
                        }

                        TapHandler {
                            onTapped: Qt.openUrlExternally("https://www.buienradar.nl")
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 132
            color: appTheme.surface
            radius: 16

            RainView {
                weatherController: root.weatherSource
                theme: appTheme
                anchors.fill: parent
                anchors.margins: 16
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
