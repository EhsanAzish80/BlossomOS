import QtQuick
import QtQuick.Controls
import Quickshell
import Blossom.Shell

ShellRoot {
    Component.onCompleted: BlossomBroker.refreshActivity()

    PanelWindow {
        id: commandBar
        anchors {
            top: true
            left: true
            right: true
        }
        implicitHeight: 52
        color: "#11151c"

        Row {
            anchors {
                fill: parent
                leftMargin: 18
                rightMargin: 18
            }
            spacing: 16

            Label {
                anchors.verticalCenter: parent.verticalCenter
                color: "#f4f7fb"
                font.bold: true
                text: "Blossom OS"
            }

            Button {
                anchors.verticalCenter: parent.verticalCenter
                text: "Request kernel identity"
                enabled: BlossomBroker.state !== "waiting" && BlossomBroker.state !== "submitting"
                onClicked: BlossomBroker.requestSystemUname()
            }

            Label {
                anchors.verticalCenter: parent.verticalCenter
                color: BlossomBroker.state === "unavailable" ? "#ff8a80" : "#b8c4d6"
                text: "Status: " + BlossomBroker.state
            }

            Button {
                anchors.verticalCenter: parent.verticalCenter
                text: "Refresh activity"
                onClicked: BlossomBroker.refreshActivity()
            }
        }
    }

    ApprovalPanel {}
    ActivityPanel {}
}
