import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import Blossom.Shell

ApplicationWindow {
    id: shellBar
    visible: true
    x: 0; y: 0
    width: screen ? screen.width : 1280
    height: 58
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color: "#f2171d27"
    title: "Blossom OS"
    property string clockText: Qt.formatDateTime(new Date(), "ddd HH:mm")

    Timer {
        interval: 30000
        running: true
        repeat: true
        onTriggered: shellBar.clockText = Qt.formatDateTime(new Date(), "ddd HH:mm")
    }

    function restoreRequestFocus() {
        if (agentButton.enabled) {
            shellBar.requestActivate()
            agentButton.forceActiveFocus(Qt.ActiveWindowFocusReason)
        }
    }

    Component.onCompleted: {
        BlossomBroker.refreshActivity()
        BlossomBroker.refreshBattery()
        BlossomBroker.refreshNetwork()
    }

    Rectangle {
        anchors.fill: parent
        color: "#f2171d27"
        border.color: "#2e3a4b"
        border.width: 1
        RowLayout {
            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
            spacing: 10
            Button {
                text: "Applications"
                activeFocusOnTab: true
                Accessible.description: "Open the Blossom OS application launcher."
                onClicked: {
                    launcherWindow.visible = !launcherWindow.visible
                    if (launcherWindow.visible) launcherWindow.requestActivate()
                }
            }
            Label { text: "Blossom OS"; color: "#f4f7fb"; font.bold: true; font.pixelSize: 17 }
            Item { Layout.fillWidth: true }
            Label {
                text: "Network: " + BlossomBroker.network.connectivity
                color: BlossomBroker.network.connectivity === "offline" ? "#ff9b93" : "#b8c4d6"
                Accessible.name: text
            }
            Label {
                text: BlossomBroker.battery.status === "present"
                    ? "Battery: " + BlossomBroker.battery.percentage + "%"
                    : BlossomBroker.battery.status === "absent" ? "AC power" : "Battery unavailable"
                color: "#b8c4d6"
                Accessible.name: text
            }
            Button {
                text: "Activity"
                onClicked: activityPanel.visible = !activityPanel.visible
                Accessible.description: "Show or hide authoritative agent activity."
            }
            Button {
                id: agentButton
                text: "Agent check"
                Accessible.name: "Request kernel identity"
                enabled: !["requesting", "waiting", "submitting", "cancelling"].includes(BlossomBroker.state)
                onClicked: BlossomBroker.requestSystemUname()
                Accessible.description: "Request the fixed kernel identity diagnostic."
            }
            Label { text: shellBar.clockText; color: "#f4f7fb"; Accessible.name: text }
            Label {
                text: BlossomBroker.state === "idle" ? "Ready" : BlossomBroker.state
                color: BlossomBroker.state === "unavailable" ? "#ff9b93" : "#8dd7c7"
                Accessible.role: Accessible.AlertMessage
                Accessible.name: "Agent status: " + text
            }
        }
    }

    ApplicationWindow {
        id: launcherWindow
        visible: false
        x: 12; y: 66
        width: Math.min(520, screen ? screen.width - 24 : 520)
        height: Math.min(570, screen ? screen.height - 84 : 570)
        flags: Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
        color: "#f2171d27"
        title: "Applications"
        Rectangle {
            anchors.fill: parent
            color: "#171d27"
            border.color: "#3b4a60"
            border.width: 1
            radius: 16
            ColumnLayout {
                anchors { fill: parent; margins: 24 }
                spacing: 14
                RowLayout {
                    Layout.fillWidth: true
                    Label { Layout.fillWidth: true; text: "Applications"; color: "#f4f7fb"; font.pixelSize: 26; font.bold: true }
                    Button { text: "Close"; onClicked: launcherWindow.hide() }
                }
                Label {
                    Layout.fillWidth: true
                    text: "Work, browse, manage files, and configure this computer."
                    color: "#aebbd0"; wrapMode: Text.WordWrap
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    columnSpacing: 12; rowSpacing: 12
                    Button { Layout.fillWidth: true; text: "Files"; Accessible.ignored: false; onClicked: { BlossomBroker.openFiles(); launcherWindow.hide() } }
                    Button { Layout.fillWidth: true; text: "Web Browser"; Accessible.ignored: false; onClicked: { BlossomBroker.openBrowser(); launcherWindow.hide() } }
                    Button { Layout.fillWidth: true; text: "Text Editor"; Accessible.ignored: false; onClicked: { BlossomBroker.openEditor(); launcherWindow.hide() } }
                    Button { Layout.fillWidth: true; text: "Terminal"; Accessible.ignored: false; onClicked: { BlossomBroker.openTerminal(); launcherWindow.hide() } }
                    Button { Layout.fillWidth: true; text: "Network Settings"; Accessible.ignored: false; onClicked: { BlossomBroker.openNetworkSettings(); launcherWindow.hide() } }
                    Button { Layout.fillWidth: true; text: "Audio Settings"; Accessible.ignored: false; onClicked: { BlossomBroker.openAudioSettings(); launcherWindow.hide() } }
                }
                Rectangle { Layout.fillWidth: true; height: 1; color: "#334155" }
                Button {
                    Layout.fillWidth: true
                    visible: BlossomBroker.liveEnvironment
                    text: "Install Blossom OS"
                    onClicked: { BlossomBroker.openInstaller(); launcherWindow.hide() }
                    Accessible.description: "Open the guarded installer. No disk is changed automatically."
                }
                RowLayout {
                    Layout.fillWidth: true
                    Button { Layout.fillWidth: true; text: "Restart"; onClicked: { launcherWindow.hide(); restartDialog.open() } }
                    Button { Layout.fillWidth: true; text: "Shut Down"; onClicked: { launcherWindow.hide(); shutdownDialog.open() } }
                }
                Label {
                    Layout.fillWidth: true; Layout.fillHeight: true
                    verticalAlignment: Text.AlignBottom
                    text: BlossomBroker.liveEnvironment
                        ? "Live session · External disks are excluded from installation targets, and installation requires a separate exact confirmation."
                        : "Installed system · your files remain local by default."
                    color: "#8190a5"; wrapMode: Text.WordWrap
                }
            }
        }
    }

    ApplicationWindow {
        id: welcomeWindow
        visible: true
        width: Math.min(780, screen ? screen.width - 64 : 780)
        height: Math.min(510, screen ? screen.height - 116 : 510)
        x: screen ? Math.round((screen.width - width) / 2) : 32
        y: screen ? Math.round((screen.height - height) / 2) : 84
        color: "#111823"
        title: "Welcome to Blossom OS"
        Rectangle {
            anchors.fill: parent
            color: "#111823"
            border.color: "#31435c"; border.width: 1; radius: 18
            ColumnLayout {
                anchors { fill: parent; margins: 48 }
                spacing: 20
                Label { text: BlossomBroker.liveEnvironment ? "LIVE SESSION" : "BLOSSOM OS"; color: "#8dd7c7"; font.bold: true }
                Label {
                    Layout.fillWidth: true
                    text: BlossomBroker.liveEnvironment ? "Welcome to Blossom OS" : "Your workspace is ready"
                    color: "#f4f7fb"; font.pixelSize: 38; font.bold: true; wrapMode: Text.WordWrap
                }
                Label {
                    Layout.fillWidth: true
                    text: BlossomBroker.liveEnvironment
                        ? "Try the browser, files, editor, terminal, network, and audio tools without changing this computer. Install only when you choose the guarded installer."
                        : "Open Applications to browse, work with files, adjust settings, or use the local-first agent surfaces."
                    color: "#b8c4d6"; font.pixelSize: 17; wrapMode: Text.WordWrap
                }
                RowLayout {
                    spacing: 12
                    Button { text: "Explore applications"; onClicked: { welcomeWindow.hide(); launcherWindow.show(); launcherWindow.requestActivate() } }
                    Button { text: "Open terminal"; onClicked: BlossomBroker.openTerminal() }
                    Button { visible: BlossomBroker.liveEnvironment; text: "Install"; onClicked: BlossomBroker.openInstaller() }
                }
                Item { Layout.fillHeight: true }
                Label { Layout.fillWidth: true; text: "Tip: press Super + Enter at any time for a terminal."; color: "#8190a5"; wrapMode: Text.WordWrap }
            }
        }
    }

    Connections {
        target: BlossomBroker
        function onStateChanged() {
            if (!["waiting", "submitting", "cancelling"].includes(BlossomBroker.state))
                Qt.callLater(shellBar.restoreRequestFocus)
        }
    }
    Dialog {
        id: restartDialog
        title: "Restart Blossom OS?"
        modal: true
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: BlossomBroker.restartSystem()
        Label { text: "Close your work before restarting."; color: "#f4f7fb" }
    }
    Dialog {
        id: shutdownDialog
        title: "Shut down Blossom OS?"
        modal: true
        standardButtons: Dialog.Cancel | Dialog.Ok
        onAccepted: BlossomBroker.powerOff()
        Label { text: "Close your work before shutting down."; color: "#f4f7fb" }
    }
    ApprovalPanel {}
    ActivityPanel { id: activityPanel }
}
