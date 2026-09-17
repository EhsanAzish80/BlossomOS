import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Blossom.Shell

ShellRoot {
    id: root
    property bool launcherVisible: false
    property bool welcomeVisible: true
    property bool activityVisible: false
    property bool powerVisible: false
    property string powerAction: ""
    property string clockText: Qt.formatDateTime(new Date(), "ddd HH:mm")

    Timer { interval: 30000; running: true; repeat: true; onTriggered: root.clockText = Qt.formatDateTime(new Date(), "ddd HH:mm") }
    Component.onCompleted: {
        BlossomBroker.refreshActivity()
        BlossomBroker.refreshBattery()
        BlossomBroker.refreshNetwork()
    }

    Variants {
        model: Quickshell.screens
        PanelWindow {
            required property var modelData
            screen: modelData
            anchors { top: true; bottom: true; left: true; right: true }
            aboveWindows: false
            focusable: false
            exclusiveZone: 0
            WlrLayershell.layer: WlrLayer.Background
            WlrLayershell.namespace: "blossom-background"
            color: "#0a111b"
            Image { anchors.fill: parent; source: "wallpaper.svg"; fillMode: Image.PreserveAspectCrop; asynchronous: true }
        }
    }

    Variants {
        model: Quickshell.screens
        PanelWindow {
            id: topBar
            required property var modelData
            screen: modelData
            anchors { top: true; left: true; right: true }
            implicitHeight: 52
            exclusiveZone: 52
            focusable: true
            WlrLayershell.namespace: "blossom-top-bar"
            color: "#e6151d29"

            RowLayout {
                anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                spacing: 12
                Button { text: "Blossom OS"; onClicked: root.launcherVisible = !root.launcherVisible; Accessible.description: "Open the Blossom OS application launcher." }
                Label { text: BlossomBroker.liveEnvironment ? "Live session" : "Workspace"; color: "#9fb0c7" }
                Item { Layout.fillWidth: true }
                Label { text: "Network: " + BlossomBroker.network.connectivity; color: BlossomBroker.network.connectivity === "offline" ? "#ff9b93" : "#c9d5e6" }
                Label { text: BlossomBroker.battery.status === "present" ? "Battery " + BlossomBroker.battery.percentage + "%" : BlossomBroker.battery.status === "absent" ? "AC power" : "Battery unavailable"; color: "#c9d5e6" }
                Label { text: root.clockText; color: "#f4f7fb"; font.bold: true }
                Button { text: "System"; onClicked: { root.powerVisible = !root.powerVisible; root.powerAction = "" } }
            }

            PopupWindow {
                anchor.window: topBar
                anchor.rect.x: 12
                anchor.rect.y: topBar.height + 8
                width: Math.min(520, topBar.width - 24)
                height: 540
                visible: root.launcherVisible
                grabFocus: true
                color: "transparent"
                Rectangle {
                    anchors.fill: parent; radius: 18; color: "#f2171d27"; border { color: "#40536d"; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 22 }; spacing: 12
                        RowLayout { Layout.fillWidth: true; Label { Layout.fillWidth: true; text: "Applications"; color: "#f4f7fb"; font.pixelSize: 25; font.bold: true }; Button { text: "Close"; onClicked: root.launcherVisible = false } }
                        Label { Layout.fillWidth: true; text: "Work normally or open the local agent when you choose."; color: "#aebbd0"; wrapMode: Text.WordWrap }
                        GridLayout {
                            Layout.fillWidth: true; columns: 2; columnSpacing: 10; rowSpacing: 10
                            Button { Layout.fillWidth: true; text: "Files"; onClicked: { BlossomBroker.openFiles(); root.launcherVisible = false } }
                            Button { Layout.fillWidth: true; text: "Web Browser"; onClicked: { BlossomBroker.openBrowser(); root.launcherVisible = false } }
                            Button { Layout.fillWidth: true; text: "Text Editor"; onClicked: { BlossomBroker.openEditor(); root.launcherVisible = false } }
                            Button { Layout.fillWidth: true; text: "Terminal"; onClicked: { BlossomBroker.openTerminal(); root.launcherVisible = false } }
                            Button { Layout.fillWidth: true; text: "Network Settings"; onClicked: { BlossomBroker.openNetworkSettings(); root.launcherVisible = false } }
                            Button { Layout.fillWidth: true; text: "Audio Settings"; onClicked: { BlossomBroker.openAudioSettings(); root.launcherVisible = false } }
                        }
                        Rectangle { Layout.fillWidth: true; height: 1; color: "#334155" }
                        Button { Layout.fillWidth: true; visible: BlossomBroker.liveEnvironment; text: "Install Blossom OS"; onClicked: { BlossomBroker.openInstaller(); root.launcherVisible = false } }
                        Label { Layout.fillWidth: true; visible: BlossomBroker.desktopMessage.length > 0; text: BlossomBroker.desktopMessage; color: BlossomBroker.desktopMessage.startsWith("Could not") ? "#ff9b93" : "#8dd7c7"; wrapMode: Text.WordWrap; Accessible.role: Accessible.AlertMessage }
                        Item { Layout.fillHeight: true }
                        Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Installation uses verified files from this media and can proceed offline." : "Installed system · files and settings are persistent."; color: "#8190a5"; wrapMode: Text.WordWrap }
                    }
                }
            }

            PopupWindow {
                anchor.window: topBar
                anchor.rect.x: Math.max(12, topBar.width - width - 12)
                anchor.rect.y: topBar.height + 8
                width: 340
                height: root.powerAction === "" ? 180 : 220
                visible: root.powerVisible
                grabFocus: true
                color: "transparent"
                Rectangle {
                    anchors.fill: parent; radius: 18; color: "#f2171d27"; border { color: "#40536d"; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 20 }; spacing: 12
                        Label { Layout.fillWidth: true; text: root.powerAction === "" ? "System" : (root.powerAction === "restart" ? "Restart Blossom OS?" : "Shut down Blossom OS?"); color: "#f4f7fb"; font.pixelSize: 20; font.bold: true; wrapMode: Text.WordWrap }
                        Label { Layout.fillWidth: true; visible: root.powerAction !== ""; text: "Save your work before continuing."; color: "#b8c4d6"; wrapMode: Text.WordWrap }
                        Label { Layout.fillWidth: true; visible: root.powerAction === ""; text: "Super+Tab switches windows · Super+Q closes the active window"; color: "#8fa0b7"; wrapMode: Text.WordWrap }
                        RowLayout {
                            Layout.fillWidth: true
                            Button { Layout.fillWidth: true; text: root.powerAction === "" ? "Restart" : "Cancel"; onClicked: root.powerAction === "" ? root.powerAction = "restart" : root.powerAction = "" }
                            Button { Layout.fillWidth: true; text: root.powerAction === "" ? "Shut Down" : "Confirm"; onClicked: { if (root.powerAction === "restart") BlossomBroker.restartSystem(); else if (root.powerAction === "shutdown") BlossomBroker.powerOff(); else root.powerAction = "shutdown" } }
                        }
                    }
                }
            }
        }
    }

    Variants {
        model: Quickshell.screens
        PanelWindow {
            id: dock
            required property var modelData
            screen: modelData
            anchors { bottom: true }
            margins { bottom: 14 }
            implicitWidth: 650
            implicitHeight: 70
            exclusiveZone: 92
            focusable: true
            WlrLayershell.namespace: "blossom-dock"
            color: "transparent"
            Rectangle {
                anchors.fill: parent; radius: 22; color: "#e617202c"; border { color: "#40536d"; width: 1 }
                RowLayout {
                    anchors { fill: parent; margins: 10 }; spacing: 8
                    Button { Layout.fillWidth: true; text: "Applications"; Accessible.ignored: false; onClicked: root.launcherVisible = !root.launcherVisible }
                    Button { text: "Files"; Accessible.ignored: false; onClicked: BlossomBroker.openFiles() }
                    Button { text: "Browser"; Accessible.ignored: false; onClicked: BlossomBroker.openBrowser() }
                    Button { text: "Terminal"; Accessible.ignored: false; onClicked: BlossomBroker.openTerminal() }
                    Button { text: "Network"; Accessible.ignored: false; onClicked: BlossomBroker.openNetworkSettings() }
                    Button { text: "Agent"; Accessible.ignored: false; onClicked: root.activityVisible = !root.activityVisible }
                }
            }
            PopupWindow {
                anchor.window: dock
                anchor.rect.x: Math.max(0, (dock.width - width) / 2)
                anchor.rect.y: -height - 10
                width: 520; height: 460
                visible: root.activityVisible
                grabFocus: true
                color: "transparent"
                Rectangle {
                    anchors.fill: parent; radius: 18; color: "#f2171d27"; border { color: "#40536d"; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 20 }; spacing: 10
                        RowLayout { Layout.fillWidth: true; Label { Layout.fillWidth: true; text: "Agent activity"; color: "#f4f7fb"; font.pixelSize: 22; font.bold: true }; Button { text: "System check"; enabled: !["requesting", "waiting", "submitting", "cancelling"].includes(BlossomBroker.state); Accessible.description: "Request the fixed kernel identity diagnostic."; onClicked: BlossomBroker.requestSystemUname() }; Button { text: "Close"; onClicked: root.activityVisible = false } }
                        Label { Layout.fillWidth: true; text: "The desktop works without an active model. Privileged agent actions still require explicit approval."; color: "#aebbd0"; wrapMode: Text.WordWrap }
                        ListView {
                            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 8; model: BlossomBroker.activity
                            delegate: Rectangle { required property var modelData; width: ListView.view.width; height: entry.implicitHeight + 16; radius: 8; color: "#222b38"; Label { id: entry; anchors { fill: parent; margins: 8 }; color: "#dbe5f5"; wrapMode: Text.Wrap; text: "#" + modelData.sequence + "  " + modelData.kind + "\n" + modelData.category } }
                        }
                    }
                }
            }
        }
    }

    Variants {
        model: Quickshell.screens
        PanelWindow {
            required property var modelData
            screen: modelData
            anchors { top: true; bottom: true; left: true; right: true }
            aboveWindows: true
            focusable: true
            exclusiveZone: 0
            visible: root.welcomeVisible && modelData === Quickshell.screens[0]
            color: "#66070b12"
            WlrLayershell.namespace: "blossom-welcome"
            Rectangle {
                anchors.centerIn: parent; width: Math.min(720, parent.width - 64); height: Math.min(470, parent.height - 120); radius: 24; color: "#f2111823"; border { color: "#506987"; width: 1 }
                ColumnLayout {
                    anchors { fill: parent; margins: 42 }; spacing: 18
                    Label { text: BlossomBroker.liveEnvironment ? "LIVE SESSION" : "BLOSSOM OS"; color: "#8dd7c7"; font.bold: true }
                    Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Try Blossom OS" : "Your desktop is ready"; color: "#f4f7fb"; font.pixelSize: 36; font.bold: true; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Browse, work with files, connect to a network, or install from the verified offline media when you are ready." : "Use Blossom as a normal computer. Open Agent when you want local assistance."; color: "#c0ccdc"; font.pixelSize: 17; wrapMode: Text.WordWrap }
                    RowLayout { Button { text: "Continue to desktop"; onClicked: root.welcomeVisible = false }; Button { text: "Network"; onClicked: BlossomBroker.openNetworkSettings() }; Button { visible: BlossomBroker.liveEnvironment; text: "Install Blossom OS"; onClicked: BlossomBroker.openInstaller() } }
                    Item { Layout.fillHeight: true }
                    Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Welcome to Blossom OS. Installing is never automatic. External disks are excluded from installation targets, and you can continue offline." : "Applications and files remain available even when the agent is not configured."; color: "#8190a5"; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; visible: BlossomBroker.desktopMessage.length > 0; text: BlossomBroker.desktopMessage; color: BlossomBroker.desktopMessage.startsWith("Could not") ? "#ff9b93" : "#8dd7c7"; wrapMode: Text.WordWrap; Accessible.role: Accessible.AlertMessage }
                }
            }
        }
    }
}
