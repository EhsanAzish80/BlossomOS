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

    function restoreRequestFocus() {
        if (requestButton.enabled) {
            commandBar.requestActivate()
            requestButton.forceActiveFocus(Qt.ActiveWindowFocusReason)
        }
    }

    onActiveChanged: {
        if (active && requestButton.enabled) {
            requestButton.forceActiveFocus(Qt.ActiveWindowFocusReason)
        }
    }

    Component.onCompleted: {
        BlossomBroker.refreshActivity()
        BlossomBroker.refreshBattery()
        BlossomBroker.refreshNetwork()
        requestActivate()
        requestButton.forceActiveFocus(Qt.ActiveWindowFocusReason)
    }

    ApplicationWindow {
        id: welcomeWindow
        visible: true
        width: Math.min(760, screen ? screen.width - 64 : 760)
        height: Math.min(500, screen ? screen.height - 116 : 500)
        x: screen ? Math.round((screen.width - width) / 2) : 32
        y: screen ? Math.round((screen.height - height) / 2) : 84
        color: "#111823"
        title: "Welcome to Blossom OS"

        Rectangle {
            anchors.fill: parent
            color: "#111823"
            border.color: "#31435c"
            border.width: 1
            radius: 16

            Column {
                anchors {
                    fill: parent
                    margins: 48
                }
                spacing: 22

                Label {
                    color: "#8dd7c7"
                    font.pixelSize: 15
                    font.bold: true
                    text: BlossomBroker.liveEnvironment ? "LIVE SESSION" : "BLOSSOM OS"
                }

                Label {
                    width: parent.width
                    color: "#f4f7fb"
                    font.pixelSize: 36
                    font.bold: true
                    wrapMode: Text.WordWrap
                    text: BlossomBroker.liveEnvironment
                        ? "Welcome to Blossom OS"
                        : "Your local-first workspace is ready"
                }

                Label {
                    width: parent.width
                    color: "#b8c4d6"
                    font.pixelSize: 17
                    lineHeight: 1.25
                    wrapMode: Text.WordWrap
                    text: BlossomBroker.liveEnvironment
                        ? "Explore the desktop without changing this computer. When you are ready, the installer will identify the internal target and require an exact confirmation before writing anything."
                        : "The desktop shell is running. Open a terminal with the button below or press Super + Enter."
                }

                Row {
                    spacing: 14

                    Button {
                        text: "Open terminal"
                        activeFocusOnTab: true
                        Accessible.name: text
                        Accessible.description: "Open the Blossom OS terminal."
                        onClicked: BlossomBroker.openTerminal()
                    }

                    Button {
                        visible: BlossomBroker.liveEnvironment
                        text: "Install Blossom OS"
                        activeFocusOnTab: true
                        Accessible.name: text
                        Accessible.description: "Open the guarded Blossom OS installer."
                        onClicked: BlossomBroker.openInstaller()
                    }

                    Button {
                        text: "Continue to desktop"
                        activeFocusOnTab: true
                        Accessible.name: text
                        onClicked: welcomeWindow.hide()
                    }
                }

                Label {
                    width: parent.width
                    color: "#8190a5"
                    font.pixelSize: 14
                    wrapMode: Text.WordWrap
                    text: BlossomBroker.liveEnvironment
                        ? "Installation is never automatic. External disks are excluded from installation targets."
                        : "System status remains available in the bar at the top of the screen."
                }
            }
        }
    }

    Connections {
        target: BlossomBroker
        function onStateChanged() {
            if (BlossomBroker.state !== "waiting"
                    && BlossomBroker.state !== "submitting"
                    && BlossomBroker.state !== "cancelling") {
                // Let the modal visibility binding unmap its Wayland surface
                // before returning activation to the command bar.
                Qt.callLater(commandBar.restoreRequestFocus)
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

        Label {
            anchors.verticalCenter: parent.verticalCenter
            color: "#b8c4d6"
            text: "Network: " + BlossomBroker.network.connectivity
            Accessible.role: Accessible.StaticText
            Accessible.name: text
            Accessible.ignored: false
        }

        Label {
            anchors.verticalCenter: parent.verticalCenter
            color: "#b8c4d6"
            text: BlossomBroker.battery.status === "present"
                ? "Battery: " + BlossomBroker.battery.percentage + "% (" + BlossomBroker.battery.state + ")"
                : BlossomBroker.battery.status === "absent" ? "Battery: none" : "Battery: unavailable"
            Accessible.role: Accessible.StaticText
            Accessible.name: text
            Accessible.ignored: false
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
