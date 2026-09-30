import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Blossom.Shell

ShellRoot {
    id: root
    property bool launcherVisible: false
    property bool welcomeVisible: BlossomBroker.onboardingRequired
    property bool activityVisible: false
    property bool agentHiddenForApproval: false
    property bool quickVisible: false
    property bool powerVisible: false
    property bool installerRequested: false
    property bool installerOpened: false
    property string installerFeedback: ""
    property string powerAction: ""
    property string clockText: Qt.formatDateTime(new Date(), "ddd HH:mm")
    // QEMU Cocoa can briefly publish the same virtio scanout twice while
    // entering macOS fullscreen. Keep one coherent desktop surface set on
    // the primary output instead of stacking duplicate exclusive-zone bars.
    readonly property var desktopScreens: Quickshell.screens.length > 0
        ? [Quickshell.screens[0]] : []

    Timer { interval: 30000; running: true; repeat: true; onTriggered: root.clockText = Qt.formatDateTime(new Date(), "ddd HH:mm") }
    Shortcut {
        sequence: "Escape"
        context: Qt.ApplicationShortcut
        onActivated: {
            root.launcherVisible = false
            root.activityVisible = false
            root.quickVisible = false
            root.powerVisible = false
            root.powerAction = ""
        }
    }
    Component.onCompleted: {
        BlossomBroker.refreshActivity()
        BlossomBroker.refreshBattery()
        BlossomBroker.refreshNetwork()
        BlossomBroker.refreshQuickStatus()
    }
    Connections {
        target: BlossomBroker
        function onStateChanged() {
            const busy = ["waiting", "submitting", "cancelling"].includes(BlossomBroker.state)
            const finished = ["unavailable", "verified", "verification_failed", "cancelled", "expired"]
                .includes(BlossomBroker.state)
            if (busy) {
                if (root.activityVisible)
                    root.agentHiddenForApproval = true
                root.activityVisible = false
                root.launcherVisible = false
                root.quickVisible = false
                root.powerVisible = false
            } else if (root.agentHiddenForApproval && finished) {
                root.agentHiddenForApproval = false
                root.activityVisible = true
            }
        }
        function onDesktopMessageChanged() {
            if (!root.installerRequested)
                return
            root.installerFeedback = BlossomBroker.desktopMessage
            if (BlossomBroker.desktopMessage === "Opened installer.")
                root.installerOpened = true
            else if (BlossomBroker.desktopMessage.startsWith("Could not"))
                root.installerRequested = false
        }
    }

    // The approval must live in the client connection that started the
    // request.  The separate accessibility host has its own D-Bus peer and
    // therefore cannot adopt or decide this pending request.
    ApprovalPanel {}

    Variants {
        model: root.desktopScreens
        PanelWindow {
            required property var modelData
            screen: modelData
            anchors { top: true; bottom: true; left: true; right: true }
            aboveWindows: false
            focusable: false
            exclusiveZone: 0
            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.layer: WlrLayer.Background
            WlrLayershell.namespace: "blossom-background"
            color: "#0a111b"
            Image { anchors.fill: parent; source: "wallpaper.svg"; fillMode: Image.PreserveAspectCrop; asynchronous: true }
        }
    }

    Variants {
        model: root.desktopScreens
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
                BlossomButton { text: "Blossom OS"; onClicked: root.launcherVisible = !root.launcherVisible; Accessible.description: "Open the Blossom OS application launcher." }
                Label { text: BlossomBroker.liveEnvironment ? "Live session" : "Workspace"; color: "#9fb0c7" }
                Item { Layout.fillWidth: true }
                BlossomIconButton { symbol: BlossomBroker.quickStatus.network === "ethernet" ? "↔" : "≋"; description: BlossomBroker.quickStatus.network === "wifi" ? "Wi-Fi connected" : BlossomBroker.quickStatus.network === "ethernet" ? "Ethernet connected" : "Network " + BlossomBroker.network.connectivity; selected: root.quickVisible; onClicked: { root.quickVisible = !root.quickVisible; root.powerVisible = false } }
                BlossomIconButton { symbol: BlossomBroker.quickStatus.muted ? "×" : "♪"; description: BlossomBroker.quickStatus.volume_percent >= 0 ? (BlossomBroker.quickStatus.muted ? "Sound muted" : "Sound " + BlossomBroker.quickStatus.volume_percent + "%") : "Sound unavailable"; selected: root.quickVisible; onClicked: { root.quickVisible = !root.quickVisible; root.powerVisible = false } }
                BlossomIconButton { symbol: "ᛒ"; description: BlossomBroker.quickStatus.bluetooth === "on" ? "Bluetooth on" : BlossomBroker.quickStatus.bluetooth === "off" ? "Bluetooth off" : "Bluetooth unavailable"; onClicked: BlossomBroker.openBluetoothSettings() }
                BlossomIconButton { symbol: BlossomBroker.battery.status === "present" ? "▰" : "ϟ"; description: BlossomBroker.battery.status === "present" ? "Battery " + BlossomBroker.battery.percentage + "%" : BlossomBroker.battery.status === "absent" ? "Connected to AC power" : "Power status unavailable"; selected: root.quickVisible; onClicked: root.quickVisible = !root.quickVisible }
                Label { text: root.clockText; color: "#f4f7fb"; font.bold: true }
                BlossomIconButton { symbol: "⏻"; description: "System menu"; selected: root.powerVisible; onClicked: { root.powerVisible = !root.powerVisible; root.quickVisible = false; root.powerAction = "" } }
            }

            PopupWindow {
                anchor.window: topBar
                anchor.rect.x: Math.max(12, topBar.width - width - 110)
                anchor.rect.y: topBar.height + 8
                width: 430
                height: 390
                visible: root.quickVisible
                grabFocus: true
                color: "transparent"
                Rectangle {
                    anchors.fill: parent; radius: 18; color: "#f2171d27"; border { color: "#40536d"; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 20 } spacing: 12
                        RowLayout { Layout.fillWidth: true; Label { Layout.fillWidth: true; text: "Quick settings"; color: "#f4f7fb"; font.pixelSize: 22; font.bold: true } BlossomIconButton { symbol: "×"; description: "Close quick settings"; onClicked: root.quickVisible = false } }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 72; radius: 12; color: "#222b38"
                            RowLayout { anchors { fill: parent; margins: 10 } Label { Layout.fillWidth: true; text: "Network\n" + BlossomBroker.quickStatus.network + " · " + BlossomBroker.network.connectivity; color: BlossomBroker.network.connectivity === "offline" ? "#ff9b93" : "#dbe5f5" } BlossomButton { text: "Wi-Fi & Ethernet"; onClicked: BlossomBroker.openNetworkSettings() } }
                        }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 72; radius: 12; color: "#222b38"
                            RowLayout { anchors { fill: parent; margins: 10 } Label { Layout.fillWidth: true; text: BlossomBroker.quickStatus.volume_percent >= 0 ? "Sound " + BlossomBroker.quickStatus.volume_percent + "%" : "Sound unavailable"; color: "#dbe5f5" } BlossomIconButton { symbol: "−"; description: "Lower volume"; onClicked: BlossomBroker.lowerVolume() } BlossomButton { text: BlossomBroker.quickStatus.muted ? "Unmute" : "Mute"; onClicked: BlossomBroker.toggleAudioMute() } BlossomIconButton { symbol: "+"; description: "Raise volume"; onClicked: BlossomBroker.raiseVolume() } BlossomButton { text: "Details"; onClicked: BlossomBroker.openAudioSettings() } }
                        }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 64; radius: 12; color: "#222b38"
                            RowLayout { anchors { fill: parent; margins: 10 } Label { Layout.fillWidth: true; text: "Bluetooth " + BlossomBroker.quickStatus.bluetooth; color: "#dbe5f5" } BlossomButton { text: "Devices"; onClicked: BlossomBroker.openBluetoothSettings() } }
                        }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 64; radius: 12; color: "#222b38"
                            RowLayout { anchors { fill: parent; margins: 10 } Label { Layout.fillWidth: true; text: "Notifications"; color: "#dbe5f5" } BlossomButton { text: "Do not disturb"; onClicked: BlossomBroker.toggleDoNotDisturb() } }
                        }
                        Label { Layout.fillWidth: true; text: BlossomBroker.battery.status === "present" ? "Battery " + BlossomBroker.battery.percentage + "% · " + BlossomBroker.battery.state : BlossomBroker.battery.status === "absent" ? "Connected to AC power" : "Battery information unavailable"; color: "#9fb0c7"; wrapMode: Text.WordWrap }
                    }
                }
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
                        anchors { fill: parent; margins: 22 } spacing: 12
                        RowLayout { Layout.fillWidth: true; Label { Layout.fillWidth: true; text: "Applications"; color: "#f4f7fb"; font.pixelSize: 25; font.bold: true } BlossomIconButton { symbol: "×"; description: "Close applications"; onClicked: root.launcherVisible = false } }
                        Label { Layout.fillWidth: true; text: "Work normally or open the local agent when you choose."; color: "#aebbd0"; wrapMode: Text.WordWrap }
                        GridLayout {
                            Layout.fillWidth: true; columns: 2; columnSpacing: 10; rowSpacing: 10
                            BlossomButton { Layout.fillWidth: true; text: "Files"; onClicked: { BlossomBroker.openFiles(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Web Browser"; onClicked: { BlossomBroker.openBrowser(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Text Editor"; onClicked: { BlossomBroker.openEditor(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Terminal"; onClicked: { BlossomBroker.openTerminal(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Network Settings"; onClicked: { BlossomBroker.openNetworkSettings(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Audio Settings"; onClicked: { BlossomBroker.openAudioSettings(); root.launcherVisible = false } }
                        }
                        Rectangle { Layout.fillWidth: true; height: 1; color: "#334155" }
                        BlossomButton { Layout.fillWidth: true; primary: true; visible: BlossomBroker.liveEnvironment; enabled: !root.installerRequested; text: root.installerRequested ? "Opening installer…" : "Install Blossom OS"; onClicked: { root.installerFeedback = "Opening the Blossom OS installer…"; root.installerRequested = true; BlossomBroker.openInstaller() } }
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
                height: 250
                visible: root.powerVisible
                grabFocus: true
                color: "transparent"
                Rectangle {
                    anchors.fill: parent; radius: 18; color: "#f2171d27"; border { color: "#40536d"; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 20 } spacing: 12
                        Label { Layout.fillWidth: true; text: "System"; color: "#f4f7fb"; font.pixelSize: 20; font.bold: true }
                        Label { Layout.fillWidth: true; text: "Super+Tab switches windows · Super+Q closes the active window"; color: "#8fa0b7"; wrapMode: Text.WordWrap }
                        BlossomButton { Layout.fillWidth: true; text: "Log Out"; onClicked: { root.powerAction = "logout"; root.powerVisible = false } }
                        BlossomButton { Layout.fillWidth: true; text: "Restart"; onClicked: { root.powerAction = "restart"; root.powerVisible = false } }
                        BlossomButton { Layout.fillWidth: true; destructive: true; text: "Shut Down"; onClicked: { root.powerAction = "shutdown"; root.powerVisible = false } }
                    }
                }
            }
        }
    }

    Variants {
        model: root.desktopScreens
        PanelWindow {
            required property var modelData
            screen: modelData
            anchors { top: true; bottom: true; left: true; right: true }
            aboveWindows: true
            focusable: true
            exclusiveZone: 0
            visible: root.powerAction !== "" && modelData === Quickshell.screens[0]
            color: "#88070b12"
            WlrLayershell.namespace: "blossom-system-confirmation"
            Rectangle {
                anchors.centerIn: parent
                width: Math.min(460, parent.width - 48)
                height: 250
                radius: 22
                color: "#f2171d27"
                border { color: "#506987"; width: 1 }
                ColumnLayout {
                    anchors { fill: parent; margins: 30 } spacing: 16
                    Label { Layout.fillWidth: true; text: root.powerAction === "restart" ? "Restart Blossom OS?" : root.powerAction === "logout" ? "Log out of Blossom OS?" : "Shut down Blossom OS?"; color: "#f4f7fb"; font.pixelSize: 25; font.bold: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; text: "Save your work before continuing."; color: "#b8c4d6"; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
                    Item { Layout.fillHeight: true }
                    RowLayout {
                        Layout.fillWidth: true; spacing: 12
                        BlossomButton { Layout.fillWidth: true; text: "Cancel"; onClicked: root.powerAction = "" }
                        BlossomButton { Layout.fillWidth: true; destructive: true; text: root.powerAction === "logout" ? "Log Out" : root.powerAction === "restart" ? "Restart" : "Shut Down"; onClicked: { if (root.powerAction === "logout") BlossomBroker.logOut(); else if (root.powerAction === "restart") BlossomBroker.restartSystem(); else BlossomBroker.powerOff() } }
                    }
                }
            }
        }
    }

    Variants {
        model: root.desktopScreens
        PanelWindow {
            id: dock
            required property var modelData
            screen: modelData
            anchors { bottom: true }
            margins { bottom: 14 }
            implicitWidth: 410
            implicitHeight: 72
            exclusiveZone: 92
            focusable: true
            WlrLayershell.namespace: "blossom-dock"
            color: "transparent"
            Rectangle {
                anchors.fill: parent; radius: 22; color: "#e617202c"; border { color: "#40536d"; width: 1 }
                RowLayout {
                    anchors { fill: parent; margins: 10 } spacing: 8
                    Item { Layout.fillWidth: true }
                    BlossomDockItem { iconSource: "icon-apps.svg"; text: "Applications"; description: "Applications"; selected: root.launcherVisible; onClicked: root.launcherVisible = !root.launcherVisible }
                    BlossomDockItem { iconSource: "icon-files.svg"; text: "Files"; description: "Files"; onClicked: BlossomBroker.openFiles() }
                    BlossomDockItem { iconSource: "icon-browser.svg"; text: "Browser"; description: "Web Browser"; onClicked: BlossomBroker.openBrowser() }
                    BlossomDockItem { iconSource: "icon-terminal.svg"; text: "Terminal"; description: "Terminal"; onClicked: BlossomBroker.openTerminal() }
                    Rectangle { width: 1; height: 34; color: "#405066" }
                    BlossomDockItem { iconSource: "icon-agent.svg"; text: "Agent"; description: "Blossom Agent"; selected: root.activityVisible; onClicked: root.activityVisible = !root.activityVisible }
                    Item { Layout.fillWidth: true }
                }
            }
            PopupWindow {
                anchor.window: dock
                anchor.rect.x: Math.max(0, (dock.width - width) / 2)
                anchor.rect.y: -height - 10
                width: Math.min(600, dock.screen.width - 24)
                height: Math.min(640, dock.screen.height - 140)
                visible: root.activityVisible
                grabFocus: true
                color: "transparent"
                Rectangle {
                    anchors.fill: parent; radius: 18; color: "#f2171d27"; border { color: "#40536d"; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 20 } spacing: 10
                        RowLayout { Layout.fillWidth: true; Label { Layout.fillWidth: true; text: "Blossom Agent"; color: "#f4f7fb"; font.pixelSize: 22; font.bold: true } BlossomButton { text: "System check"; enabled: !["requesting", "waiting", "submitting", "cancelling"].includes(BlossomBroker.state); Accessible.description: "Request the fixed kernel identity diagnostic."; onClicked: BlossomBroker.requestSystemUname() } BlossomIconButton { symbol: "×"; description: "Close Blossom Agent"; onClicked: root.activityVisible = false } }
                        Label {
                            Layout.fillWidth: true
                            text: BlossomBroker.state === "requesting" ? "The local model is preparing a proposal…"
                                : BlossomBroker.state === "waiting" ? "Review the exact proposed effect in the approval window."
                                : BlossomBroker.state === "submitting" ? "Authenticating and applying the approved effect…"
                                : BlossomBroker.state === "unavailable" ? BlossomBroker.failureReason + " Nothing was applied."
                                : "Ask the local agent for one bounded action. Proposed effects never run without exact approval."
                            color: BlossomBroker.state === "unavailable" ? "#ff9b93" : "#aebbd0"
                            wrapMode: Text.WordWrap
                            Accessible.role: Accessible.AlertMessage
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: 112
                            radius: 12
                            color: "#222b38"
                            border { color: agentPrompt.activeFocus ? "#8dd7c7" : "#40536d"; width: 1 }
                            TextArea {
                                id: agentPrompt
                                anchors { fill: parent; margins: 8 }
                                placeholderText: "What should Blossom do?"
                                color: "#f4f7fb"
                                placeholderTextColor: "#8190a5"
                                selectionColor: "#2f8f83"
                                selectedTextColor: "#ffffff"
                                background: Rectangle {
                                    color: "transparent"
                                }
                                wrapMode: TextEdit.Wrap
                                selectByMouse: true
                                enabled: !["requesting", "waiting", "submitting", "cancelling"].includes(BlossomBroker.state)
                                Accessible.name: "Agent request"
                                Accessible.description: "Describe one bounded action for the local Blossom agent."
                            }
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Item { Layout.fillWidth: true }
                            Label { text: agentPrompt.length + " / 4096 characters (UTF-8 byte limit also applies)"; color: agentPrompt.length > 4096 ? "#ff9b93" : "#8190a5" }
                            BlossomButton {
                                text: BlossomBroker.state === "requesting" ? "Thinking…" : "Send"
                                primary: true
                                enabled: agentPrompt.text.trim().length > 0 && agentPrompt.length <= 4096
                                    && !["requesting", "waiting", "submitting", "cancelling"].includes(BlossomBroker.state)
                                Accessible.description: "Send this request to the local model through the Blossom gateway."
                                onClicked: BlossomBroker.requestAgentTurn(agentPrompt.text)
                            }
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: agentStatus.implicitHeight + 18
                            radius: 9
                            color: BlossomBroker.state === "waiting" ? "#213b3a"
                                : BlossomBroker.state === "unavailable" ? "#442b31" : "#222b38"
                            Label {
                                id: agentStatus
                                anchors { fill: parent; margins: 9 }
                                text: BlossomBroker.state === "requesting" ? "Request sent · waiting for the local model…"
                                    : BlossomBroker.state === "waiting" ? "Approval required · review the exact proposal in the approval window."
                                    : BlossomBroker.state === "submitting" ? "Approval received · authenticating and verifying the effect…"
                                    : BlossomBroker.state === "verified" ? "Completed and verified."
                                    : BlossomBroker.state === "denied" ? "Denied · nothing was applied."
                                    : BlossomBroker.state === "cancelled" ? "Cancelled · nothing was applied."
                                    : BlossomBroker.state === "expired" ? "Expired · nothing was applied."
                                    : BlossomBroker.state === "verification_failed" ? "Verification failed · review the activity below."
                                    : BlossomBroker.state === "unavailable" ? "Request failed closed · " + BlossomBroker.failureReason
                                    : "Ready"
                                color: BlossomBroker.state === "waiting" || BlossomBroker.state === "verified"
                                    ? "#8dd7c7" : BlossomBroker.state === "unavailable" || BlossomBroker.state === "verification_failed"
                                    ? "#ff9b93" : "#dbe5f5"
                                wrapMode: Text.WordWrap
                                Accessible.role: Accessible.AlertMessage
                            }
                        }
                        Label { Layout.fillWidth: true; text: "Recent authoritative activity"; color: "#8dd7c7"; font.bold: true }
                        ListView {
                            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 8; model: BlossomBroker.activity
                            delegate: Rectangle { required property var modelData; width: ListView.view.width; height: entry.implicitHeight + 16; radius: 8; color: "#222b38"; Label { id: entry; anchors { fill: parent; margins: 8 } color: "#dbe5f5"; wrapMode: Text.Wrap; text: "#" + modelData.sequence + "  " + modelData.kind + "\n" + modelData.category } }
                        }
                    }
                }
            }
        }
    }

    Variants {
        model: root.desktopScreens
        PanelWindow {
            required property var modelData
            screen: modelData
            anchors { top: true; bottom: true; left: true; right: true }
            aboveWindows: true
            focusable: true
            exclusiveZone: 0
            visible: root.welcomeVisible && !root.installerOpened && modelData === Quickshell.screens[0]
            color: "#66070b12"
            WlrLayershell.namespace: "blossom-welcome"
            Rectangle {
                anchors.centerIn: parent; width: Math.min(720, parent.width - 64); height: Math.min(470, parent.height - 120); radius: 24; color: "#f2111823"; border { color: "#506987"; width: 1 }
                ColumnLayout {
                    anchors { fill: parent; margins: 42 } spacing: 18
                    Label { text: BlossomBroker.liveEnvironment ? "LIVE SESSION" : "BLOSSOM OS"; color: "#8dd7c7"; font.bold: true }
                    Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Try Blossom OS" : "Your desktop is ready"; color: "#f4f7fb"; font.pixelSize: 36; font.bold: true; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Browse, work with files, connect to a network, or install from the verified offline media when you are ready." : "Use Blossom as a normal computer. Open Agent when you want local assistance."; color: "#c0ccdc"; font.pixelSize: 17; wrapMode: Text.WordWrap }
                    RowLayout { BlossomButton { text: "Continue to desktop"; onClicked: BlossomBroker.dismissOnboarding() } BlossomButton { text: "Network"; onClicked: BlossomBroker.openNetworkSettings() } BlossomButton { primary: true; visible: BlossomBroker.liveEnvironment; enabled: !root.installerRequested; text: root.installerRequested ? "Opening installer…" : "Install Blossom OS"; onClicked: { root.installerFeedback = "Opening the Blossom OS installer…"; root.installerRequested = true; BlossomBroker.openInstaller() } } }
                    Label { Layout.fillWidth: true; visible: root.installerFeedback.length > 0; text: root.installerFeedback; color: root.installerFeedback.startsWith("Could not") ? "#ff9b93" : "#8dd7c7"; wrapMode: Text.WordWrap; Accessible.role: Accessible.AlertMessage }
                    Item { Layout.fillHeight: true }
                    Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Welcome to Blossom OS. Installing is never automatic. External disks are excluded from installation targets, and you can continue offline." : "Applications and files remain available even when the agent is not configured."; color: "#8190a5"; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; visible: BlossomBroker.desktopMessage.length > 0; text: BlossomBroker.desktopMessage; color: BlossomBroker.desktopMessage.startsWith("Could not") ? "#ff9b93" : "#8dd7c7"; wrapMode: Text.WordWrap; Accessible.role: Accessible.AlertMessage }
                }
            }
        }
    }
}
