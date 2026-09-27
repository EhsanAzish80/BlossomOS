import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Blossom.Shell
import "Petal"

ShellRoot {
    id: root
    property bool launcherVisible: false
    property bool welcomeVisible: BlossomBroker.onboardingRequired
    property bool activityVisible: false
    property bool quickVisible: false
    property bool powerVisible: false
    property bool installerRequested: false
    property string powerAction: ""
    property string clockText: Qt.formatDateTime(new Date(), "ddd HH:mm")

    // Presentation names for the closed ShellActivityCategory enum.
    readonly property var activityLabels: ({
        accepted: "Request received", policy_ask: "Needs your approval", policy_allow: "Allowed by policy",
        policy_denied: "Blocked by policy", approval_issued: "Waiting for you", approval_rejected: "Approval rejected",
        approved_once: "Approved once", denied: "Denied by you", cancelled: "Cancelled", expired: "Expired",
        started: "Running", execution_finished: "Finished", execution_failed: "Failed", verified: "Verified",
        verification_failed: "Verification failed", indeterminate: "Outcome unknown", read_started: "Reading",
        read_finished: "Read finished", read_failed: "Read failed"
    })
    function activityLabel(category) { return activityLabels[category] ?? String(category) }
    function activityTone(category) {
        if (category === "verified") return "verified"
        if (["policy_denied", "approval_rejected", "denied", "execution_failed", "verification_failed", "read_failed"].includes(category)) return "danger"
        if (["policy_ask", "approval_issued"].includes(category)) return "decision"
        return "neutral"
    }
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
            color: Theme.base
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
            color: Theme.barFill

            RowLayout {
                anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                spacing: 12
                BlossomButton { text: "Blossom OS"; onClicked: root.launcherVisible = !root.launcherVisible; Accessible.description: "Open the Blossom OS application launcher." }
                Label { text: BlossomBroker.liveEnvironment ? "Live session" : "Workspace"; color: Theme.textSecondary }
                Item { Layout.fillWidth: true }
                BlossomIconButton { iconSource: BlossomBroker.quickStatus.network === "ethernet" ? "Petal/icons/ethernet.svg" : BlossomBroker.network.connectivity === "offline" ? "Petal/icons/offline.svg" : "Petal/icons/wifi.svg"; description: BlossomBroker.quickStatus.network === "wifi" ? "Wi-Fi connected" : BlossomBroker.quickStatus.network === "ethernet" ? "Ethernet connected" : "Network " + BlossomBroker.network.connectivity; selected: root.quickVisible; onClicked: { root.quickVisible = !root.quickVisible; root.powerVisible = false } }
                BlossomIconButton { iconSource: BlossomBroker.quickStatus.muted ? "Petal/icons/mute.svg" : "Petal/icons/volume.svg"; description: BlossomBroker.quickStatus.volume_percent >= 0 ? (BlossomBroker.quickStatus.muted ? "Sound muted" : "Sound " + BlossomBroker.quickStatus.volume_percent + "%") : "Sound unavailable"; selected: root.quickVisible; onClicked: { root.quickVisible = !root.quickVisible; root.powerVisible = false } }
                BlossomIconButton { iconSource: "Petal/icons/bluetooth.svg"; description: BlossomBroker.quickStatus.bluetooth === "on" ? "Bluetooth on" : BlossomBroker.quickStatus.bluetooth === "off" ? "Bluetooth off" : "Bluetooth unavailable"; onClicked: BlossomBroker.openBluetoothSettings() }
                BlossomIconButton { iconSource: BlossomBroker.battery.status === "present" ? "Petal/icons/battery.svg" : "Petal/icons/ac-power.svg"; description: BlossomBroker.battery.status === "present" ? "Battery " + BlossomBroker.battery.percentage + "%" : BlossomBroker.battery.status === "absent" ? "Connected to AC power" : "Power status unavailable"; selected: root.quickVisible; onClicked: root.quickVisible = !root.quickVisible }
                Label { text: root.clockText; color: Theme.text; font.weight: Font.Medium; font.pixelSize: Theme.label }
                BlossomIconButton { iconSource: "Petal/icons/power.svg"; description: "System menu"; selected: root.powerVisible; onClicked: { root.powerVisible = !root.powerVisible; root.quickVisible = false; root.powerAction = "" } }
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
                    anchors.fill: parent; radius: Theme.radiusPanel; color: Theme.panelFill; border { color: Theme.line; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 20 } spacing: 12
                        RowLayout { Layout.fillWidth: true; Label { Layout.fillWidth: true; text: "Quick settings"; color: Theme.text; font.pixelSize: Theme.title; font.weight: Font.DemiBold } BlossomIconButton { iconSource: "Petal/icons/close.svg"; description: "Close quick settings"; onClicked: root.quickVisible = false } }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 72; radius: Theme.radiusTile; color: Theme.raised
                            RowLayout { anchors { fill: parent; margins: 10 } Label { Layout.fillWidth: true; text: "Network\n" + BlossomBroker.quickStatus.network + " · " + BlossomBroker.network.connectivity; color: BlossomBroker.network.connectivity === "offline" ? Theme.danger : Theme.text } BlossomButton { text: "Wi-Fi & Ethernet"; onClicked: BlossomBroker.openNetworkSettings() } }
                        }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 72; radius: Theme.radiusTile; color: Theme.raised
                            RowLayout { anchors { fill: parent; margins: 10 } Label { Layout.fillWidth: true; text: BlossomBroker.quickStatus.volume_percent >= 0 ? "Sound " + BlossomBroker.quickStatus.volume_percent + "%" : "Sound unavailable"; color: Theme.text } BlossomIconButton { iconSource: "Petal/icons/minus.svg"; description: "Lower volume"; onClicked: BlossomBroker.lowerVolume() } BlossomButton { text: BlossomBroker.quickStatus.muted ? "Unmute" : "Mute"; onClicked: BlossomBroker.toggleAudioMute() } BlossomIconButton { iconSource: "Petal/icons/plus.svg"; description: "Raise volume"; onClicked: BlossomBroker.raiseVolume() } BlossomButton { text: "Details"; onClicked: BlossomBroker.openAudioSettings() } }
                        }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 64; radius: Theme.radiusTile; color: Theme.raised
                            RowLayout { anchors { fill: parent; margins: 10 } Label { Layout.fillWidth: true; text: "Bluetooth " + BlossomBroker.quickStatus.bluetooth; color: Theme.text } BlossomButton { text: "Devices"; onClicked: BlossomBroker.openBluetoothSettings() } }
                        }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 64; radius: Theme.radiusTile; color: Theme.raised
                            RowLayout { anchors { fill: parent; margins: 10 } Label { Layout.fillWidth: true; text: "Notifications"; color: Theme.text } BlossomButton { text: "Do not disturb"; onClicked: BlossomBroker.toggleDoNotDisturb() } }
                        }
                        Label { Layout.fillWidth: true; text: BlossomBroker.battery.status === "present" ? "Battery " + BlossomBroker.battery.percentage + "% · " + BlossomBroker.battery.state : BlossomBroker.battery.status === "absent" ? "Connected to AC power" : "Battery information unavailable"; color: Theme.textSecondary; wrapMode: Text.WordWrap }
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
                    anchors.fill: parent; radius: Theme.radiusPanel; color: Theme.panelFill; border { color: Theme.line; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 22 } spacing: 12
                        RowLayout { Layout.fillWidth: true; Label { Layout.fillWidth: true; text: "Applications"; color: Theme.text; font.pixelSize: Theme.headline; font.weight: Font.DemiBold } BlossomIconButton { iconSource: "Petal/icons/close.svg"; description: "Close applications"; onClicked: root.launcherVisible = false } }
                        Label { Layout.fillWidth: true; text: "Work normally or open the local agent when you choose."; color: Theme.textSecondary; wrapMode: Text.WordWrap }
                        GridLayout {
                            Layout.fillWidth: true; columns: 2; columnSpacing: 10; rowSpacing: 10
                            BlossomButton { Layout.fillWidth: true; text: "Files"; onClicked: { BlossomBroker.openFiles(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Web Browser"; onClicked: { BlossomBroker.openBrowser(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Text Editor"; onClicked: { BlossomBroker.openEditor(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Terminal"; onClicked: { BlossomBroker.openTerminal(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Network Settings"; onClicked: { BlossomBroker.openNetworkSettings(); root.launcherVisible = false } }
                            BlossomButton { Layout.fillWidth: true; text: "Audio Settings"; onClicked: { BlossomBroker.openAudioSettings(); root.launcherVisible = false } }
                        }
                        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
                        BlossomButton { Layout.fillWidth: true; primary: true; visible: BlossomBroker.liveEnvironment; text: "Install Blossom OS"; onClicked: { root.launcherVisible = false; root.installerRequested = true; BlossomBroker.openInstaller() } }
                        Label { Layout.fillWidth: true; visible: BlossomBroker.desktopMessage.length > 0; text: BlossomBroker.desktopMessage; color: BlossomBroker.desktopMessage.startsWith("Could not") ? Theme.danger : Theme.verified; wrapMode: Text.WordWrap; Accessible.role: Accessible.AlertMessage }
                        Item { Layout.fillHeight: true }
                        Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Installation uses verified files from this media and can proceed offline." : "Installed system · files and settings are persistent."; color: Theme.textTertiary; wrapMode: Text.WordWrap }
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
                    anchors.fill: parent; radius: Theme.radiusPanel; color: Theme.panelFill; border { color: Theme.line; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 20 } spacing: 12
                        Label { Layout.fillWidth: true; text: "System"; color: Theme.text; font.pixelSize: Theme.title; font.weight: Font.DemiBold }
                        Label { Layout.fillWidth: true; text: "Super+Tab switches windows · Super+Q closes the active window"; color: Theme.textTertiary; wrapMode: Text.WordWrap }
                        BlossomButton { Layout.fillWidth: true; text: "Log Out"; onClicked: { root.powerAction = "logout"; root.powerVisible = false } }
                        BlossomButton { Layout.fillWidth: true; text: "Restart"; onClicked: { root.powerAction = "restart"; root.powerVisible = false } }
                        BlossomButton { Layout.fillWidth: true; destructive: true; text: "Shut Down"; onClicked: { root.powerAction = "shutdown"; root.powerVisible = false } }
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
            visible: root.powerAction !== "" && modelData === Quickshell.screens[0]
            color: Theme.scrim
            WlrLayershell.namespace: "blossom-system-confirmation"
            Rectangle {
                anchors.centerIn: parent
                width: Math.min(460, parent.width - 48)
                height: 250
                radius: Theme.radiusDock
                color: Theme.panelFill
                border { color: Theme.lineStrong; width: 1 }
                ColumnLayout {
                    anchors { fill: parent; margins: 30 } spacing: 16
                    Label { Layout.fillWidth: true; text: root.powerAction === "restart" ? "Restart Blossom OS?" : root.powerAction === "logout" ? "Log out of Blossom OS?" : "Shut down Blossom OS?"; color: Theme.text; font.pixelSize: Theme.headline; font.weight: Font.DemiBold; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; text: "Save your work before continuing."; color: Theme.textSecondary; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
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
        model: Quickshell.screens
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
                anchors.fill: parent; radius: Theme.radiusDock; color: Theme.barFill; border { color: Theme.line; width: 1 }
                RowLayout {
                    anchors { fill: parent; margins: 10 } spacing: 8
                    Item { Layout.fillWidth: true }
                    BlossomDockItem { iconSource: "icon-apps.svg"; text: "Applications"; description: "Applications"; selected: root.launcherVisible; onClicked: root.launcherVisible = !root.launcherVisible }
                    BlossomDockItem { iconSource: "icon-files.svg"; text: "Files"; description: "Files"; onClicked: BlossomBroker.openFiles() }
                    BlossomDockItem { iconSource: "icon-browser.svg"; text: "Browser"; description: "Web Browser"; onClicked: BlossomBroker.openBrowser() }
                    BlossomDockItem { iconSource: "icon-terminal.svg"; text: "Terminal"; description: "Terminal"; onClicked: BlossomBroker.openTerminal() }
                    Rectangle { width: 1; height: 32; color: Theme.line }
                    BlossomDockItem { iconSource: "icon-agent.svg"; accent: true; text: "Agent"; description: "Blossom Agent"; selected: root.activityVisible; onClicked: root.activityVisible = !root.activityVisible }
                    Item { Layout.fillWidth: true }
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
                    anchors.fill: parent; radius: Theme.radiusPanel; color: Theme.panelFill; border { color: Theme.line; width: 1 }
                    ColumnLayout {
                        anchors { fill: parent; margins: 20 } spacing: 10
                        RowLayout { Layout.fillWidth: true; Label { Layout.fillWidth: true; text: "Agent activity"; color: Theme.text; font.pixelSize: Theme.title; font.weight: Font.DemiBold } BlossomButton { text: "System check"; enabled: !["requesting", "waiting", "submitting", "cancelling"].includes(BlossomBroker.state); Accessible.description: "Request the fixed kernel identity diagnostic."; onClicked: BlossomBroker.requestSystemUname() } BlossomIconButton { iconSource: "Petal/icons/close.svg"; description: "Close agent activity"; onClicked: root.activityVisible = false } }
                        Label { Layout.fillWidth: true; text: "The desktop works without an active model. Privileged agent actions still require explicit approval."; color: Theme.textSecondary; wrapMode: Text.WordWrap }
                        ListView {
                            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 8; model: BlossomBroker.activity
                            delegate: Rectangle {
                                id: activityRow
                                required property var modelData
                                readonly property string tone: root.activityTone(modelData.category)
                                width: ListView.view.width
                                height: Math.max(52, entryText.implicitHeight + 20)
                                radius: Theme.radiusControl
                                color: Theme.raised
                                RowLayout {
                                    anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                                    spacing: 12
                                    Rectangle {
                                        Layout.preferredWidth: 28
                                        Layout.preferredHeight: 28
                                        radius: 14
                                        color: activityRow.tone === "verified" ? Theme.verifiedTint : activityRow.tone === "danger" ? Theme.dangerTint : activityRow.tone === "decision" ? Theme.decisionTint : Theme.hover
                                        Image {
                                            anchors.centerIn: parent
                                            width: 16
                                            height: 16
                                            sourceSize.width: 32
                                            sourceSize.height: 32
                                            source: activityRow.tone === "verified" ? "Petal/icons/check.svg" : activityRow.tone === "danger" ? "Petal/icons/denied.svg" : activityRow.tone === "decision" ? "Petal/icons/pending.svg" : "Petal/icons/clock.svg"
                                        }
                                    }
                                    Column {
                                        id: entryText
                                        Layout.fillWidth: true
                                        spacing: 2
                                        Label { width: parent.width; text: root.activityLabel(activityRow.modelData.category); color: Theme.text; font.pixelSize: Theme.body; font.weight: Font.Medium; elide: Text.ElideRight }
                                        Label { width: parent.width; text: activityRow.modelData.kind + " · #" + activityRow.modelData.sequence; color: Theme.textTertiary; font.family: Theme.mono; font.pixelSize: Theme.caption; elide: Text.ElideRight }
                                    }
                                }
                            }
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
            visible: root.welcomeVisible && !root.installerRequested && modelData === Quickshell.screens[0]
            color: "#990e0c10"
            WlrLayershell.namespace: "blossom-welcome"
            Rectangle {
                anchors.centerIn: parent; width: Math.min(720, parent.width - 64); height: Math.min(470, parent.height - 120); radius: Theme.radiusDock; color: Theme.panelFill; border { color: Theme.lineStrong; width: 1 }
                ColumnLayout {
                    anchors { fill: parent; margins: 42 } spacing: 18
                    Label { text: BlossomBroker.liveEnvironment ? "LIVE SESSION" : "BLOSSOM OS"; color: Theme.blossom; font.pixelSize: Theme.caption; font.letterSpacing: 1.2; font.weight: Font.DemiBold }
                    Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Try Blossom OS" : "Your desktop is ready"; color: Theme.text; font.pixelSize: Theme.display; font.weight: Font.DemiBold; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Browse, work with files, connect to a network, or install from the verified offline media when you are ready." : "Use Blossom as a normal computer. Open Agent when you want local assistance."; color: Theme.textSecondary; font.pixelSize: Theme.bodyLarge; wrapMode: Text.WordWrap }
                    RowLayout { BlossomButton { text: "Continue to desktop"; onClicked: BlossomBroker.dismissOnboarding() } BlossomButton { text: "Network"; onClicked: BlossomBroker.openNetworkSettings() } BlossomButton { primary: true; visible: BlossomBroker.liveEnvironment; text: "Install Blossom OS"; onClicked: { root.installerRequested = true; BlossomBroker.openInstaller() } } }
                    Item { Layout.fillHeight: true }
                    Label { Layout.fillWidth: true; text: BlossomBroker.liveEnvironment ? "Welcome to Blossom OS. Installing is never automatic. External disks are excluded from installation targets, and you can continue offline." : "Applications and files remain available even when the agent is not configured."; color: Theme.textTertiary; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; visible: BlossomBroker.desktopMessage.length > 0; text: BlossomBroker.desktopMessage; color: BlossomBroker.desktopMessage.startsWith("Could not") ? Theme.danger : Theme.verified; wrapMode: Text.WordWrap; Accessible.role: Accessible.AlertMessage }
                }
            }
        }
    }
}
