import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Blossom.Shell

PanelWindow {
    anchors {
        top: true
        right: true
        bottom: true
    }
    margins.top: 60
    implicitWidth: 340
    exclusiveZone: 0
    color: "#f2171d27"

    ColumnLayout {
        anchors {
            fill: parent
            margins: 16
        }
        spacing: 10

        Label {
            color: "#f4f7fb"
            font.bold: true
            font.pixelSize: 18
            text: "Authoritative activity"
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 8
            model: BlossomBroker.activity

            delegate: Rectangle {
                required property var modelData
                width: ListView.view.width
                height: activityText.implicitHeight + 16
                radius: 8
                color: "#222b38"

                Label {
                    id: activityText
                    anchors {
                        fill: parent
                        margins: 8
                    }
                    color: "#dbe5f5"
                    textFormat: Text.PlainText
                    wrapMode: Text.Wrap
                    text: "Audit sequence #" + modelData.sequence + "  " + modelData.kind + "\n" +
                          modelData.category + "  ·  " + modelData.request_id
                }
            }
        }
    }
}
