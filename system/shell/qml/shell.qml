import QtQuick
import QtQuick.Controls
import QtQuick.Window
import Blossom.Shell

ApplicationWindow {
    id: commandBar
    visible: true
    x: 0
    y: 0
    width: screen ? screen.width : 800
    height: 52
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color: "#11151c"
    title: "Blossom OS"

    onActiveChanged: {
        if (active && requestButton.enabled) {
            requestButton.forceActiveFocus(Qt.ActiveWindowFocusReason)
        }
    }

    Component.onCompleted: {
        BlossomBroker.refreshActivity()
        requestActivate()
        requestButton.forceActiveFocus(Qt.ActiveWindowFocusReason)
    }

    Connections {
        target: BlossomBroker
        function onStateChanged() {
            if (BlossomBroker.state !== "waiting"
                    && BlossomBroker.state !== "submitting"
                    && BlossomBroker.state !== "cancelling") {
                commandBar.requestActivate()
                requestButton.forceActiveFocus(Qt.ActiveWindowFocusReason)
            }
        }
    }

    Row {
        anchors {
            fill: parent
            leftMargin: 18
            rightMargin: 18
        }
        spacing: 16
        Accessible.role: Accessible.Grouping
        Accessible.name: "Blossom OS controls"
        Accessible.ignored: false

        Label {
            anchors.verticalCenter: parent.verticalCenter
            color: "#f4f7fb"
            font.bold: true
            text: "Blossom OS"
        }

        Button {
            id: requestButton
            anchors.verticalCenter: parent.verticalCenter
            text: "Request kernel identity"
            enabled: BlossomBroker.state !== "requesting"
                && BlossomBroker.state !== "waiting"
                && BlossomBroker.state !== "submitting"
                && BlossomBroker.state !== "cancelling"
            activeFocusOnTab: true
            KeyNavigation.tab: refreshButton
            KeyNavigation.backtab: refreshButton
            Accessible.name: text
            Accessible.description: "Request the fixed kernel identity diagnostic."
            Accessible.role: Accessible.Button
            Accessible.ignored: false
            onClicked: BlossomBroker.requestSystemUname()
        }

        Label {
            id: statusLabel
            anchors.verticalCenter: parent.verticalCenter
            color: BlossomBroker.state === "unavailable" ? "#ff8a80" : "#b8c4d6"
            text: "Status: " + BlossomBroker.state
            Accessible.role: Accessible.AlertMessage
            Accessible.name: text
            Accessible.ignored: false
        }

        Button {
            id: refreshButton
            anchors.verticalCenter: parent.verticalCenter
            text: "Refresh activity"
            activeFocusOnTab: true
            KeyNavigation.tab: requestButton
            KeyNavigation.backtab: requestButton
            Accessible.name: text
            Accessible.description: "Refresh the bounded authoritative activity list."
            Accessible.role: Accessible.Button
            Accessible.ignored: false
            onClicked: BlossomBroker.refreshActivity()
        }
    }

    ApprovalPanel {}
    ActivityPanel {}
}
