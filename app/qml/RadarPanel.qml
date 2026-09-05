import QtQuick

Item {
    id: radarPanel

    required property var radarController
    required property Theme theme

    Item {
        id: mapContainer
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: controlsContainer.top
        anchors.bottomMargin: radarPanel.theme.spaceMd

        Image {
            id: basemap
            anchors.fill: parent
            source: "../assets/basemap-nl.png"
            fillMode: Image.PreserveAspectCrop
            // The marker below is positioned in the raster's own coordinates, and
            // PreserveAspectCrop scales the raster past the item on one axis, so a
            // location outside the visible crop has to be cut off with it.
            clip: true

            // PreserveAspectCrop paints the source scaled by max(w/sw, h/sh) and
            // centred, so paintedWidth/paintedHeight are the on-screen size of the
            // whole raster and these offsets are its top-left corner. Anchoring the
            // marker to them is what keeps it on its coastline as the window resizes.
            readonly property real rasterOriginX: (basemap.width - basemap.paintedWidth) / 2
            readonly property real rasterOriginY: (basemap.height - basemap.paintedHeight) / 2

            Image {
                objectName: "radarOverlay"
                anchors.fill: parent
                source: radarPanel.radarController.currentFrameUrl
                fillMode: Image.PreserveAspectCrop
                visible: radarPanel.radarController.frameCount > 0
            }

            // Rust projects the configured latitude/longitude onto the radar
            // raster and hands QML two fractions; everything here is geometry.
            Item {
                id: locationMarker
                width: radarPanel.theme.spaceLg
                height: radarPanel.theme.spaceLg
                x: basemap.rasterOriginX + radarPanel.radarController.markerXFraction * basemap.paintedWidth - locationMarker.width / 2
                y: basemap.rasterOriginY + radarPanel.radarController.markerYFraction * basemap.paintedHeight - locationMarker.height / 2
                visible: radarPanel.radarController.markerVisible && basemap.status === Image.Ready

                Rectangle {
                    anchors.fill: parent
                    radius: width / 2
                    color: radarPanel.theme.background
                    opacity: 0.55
                }

                Rectangle {
                    anchors.centerIn: parent
                    width: radarPanel.theme.spaceSm
                    height: radarPanel.theme.spaceSm
                    radius: width / 2
                    color: radarPanel.theme.accent
                    border.width: 2
                    border.color: radarPanel.theme.buttonText
                }
            }
        }

        RadarLegend {
            radarController: radarPanel.radarController
            theme: radarPanel.theme
        }

        // Not the error slot, and deliberately not centred with it. A location
        // outside the Dutch raster leaves this map painting its base and nothing
        // else, which is indistinguishable from a failed frame request — so the
        // one thing worth saying is that the emptiness is the configuration
        // rather than a fault. It is a standing statement, not a failure, so it
        // takes the text colour instead of the error colour and sits at the top,
        // where it cannot collide with a refresh error arriving underneath it.
        ScrimNotice {
            objectName: "coverageNotice"
            theme: radarPanel.theme
            text: radarPanel.radarController.coverageNotice
            color: radarPanel.theme.text
            maximumLineCount: 2
            anchors.top: parent.top
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.topMargin: radarPanel.theme.spaceLg
        }

        ScrimNotice {
            objectName: "radarError"
            theme: radarPanel.theme
            text: radarPanel.radarController.status
            color: radarPanel.theme.error
            maximumLineCount: 3
            anchors.centerIn: parent
        }
    }

    RadarTransport {
        id: controlsContainer
        radarController: radarPanel.radarController
        theme: radarPanel.theme
    }
}
