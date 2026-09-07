import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import Blossom.Shell

Window {
    visible: true
    width: Math.min(340, screen ? screen.width : 800)
    height: Math.max(1, (screen ? screen.height : 600) - 60)
    x: Math.max(0, (screen ? screen.width : 800) - width)
    y: 60
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color: "#f2171d27"

    ColumnLayout {
        anchors {
            fill: parent
            margins: 16
        }
        spacing: 10
        Accessible.role: Accessible.Grouping
        Accessible.name: "Authoritative activity"
        Accessible.ignored: false

        Label {
            color: "#f4f7fb"
            font.bold: true
            font.pixelSize: 18
            text: "Authoritative activity"
            Accessible.role: Accessible.Heading
            Accessible.name: text
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 8
            model: BlossomBroker.activity
            onCountChanged: positionViewAtEnd()

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
                    Accessible.role: Accessible.StaticText
                    Accessible.name: text
                }
            }
        }
    }
}
