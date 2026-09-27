import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root
    width: 1060
    height: 720
    minimumWidth: 900
    minimumHeight: 640
    visible: true
    title: "Install Blossom OS"
    color: "#09111c"
    property int step: 0
    property var steps: ["Welcome", "Language", "Network", "Disk", "Account", "Time zone", "Review", "Install"]
    property bool completed: false
    property var reservedUsers: ["bin", "daemon", "dbus", "nobody", "root", "systemd", "wheel"]
    property bool accountValid: fullName.text.trim().length > 0 && /^[a-z][a-z0-9_-]{0,30}$/.test(username.text) && reservedUsers.indexOf(username.text) < 0 && password.text.length >= 12 && /[A-Za-z]/.test(password.text) && /[^A-Za-z]/.test(password.text) && password.text === passwordConfirmation.text && password.text.toLowerCase().indexOf(username.text.toLowerCase()) < 0
    onClosing: function(close) { if (Installer.busy) close.accepted = false }

    header: Rectangle {
        height: 72; color: "#111c2a"
        RowLayout {
            anchors { fill: parent; leftMargin: 28; rightMargin: 28 }
            Label { text: "BLOSSOM OS"; color: "#83ddca"; font.bold: true }
            Label { Layout.fillWidth: true; text: root.steps[root.step]; color: "white"; font.pixelSize: 22; horizontalAlignment: Text.AlignHCenter }
            Label { text: (root.step + 1) + " of " + root.steps.length; color: "#9babc1" }
        }
    }

    RowLayout {
        anchors {
            fill: parent
            margins: 28
        }
        spacing: 28
        Rectangle {
            Layout.preferredWidth: 220; Layout.fillHeight: true; radius: 20; color: "#101a27"
            ColumnLayout {
                anchors {
                    fill: parent
                    margins: 22
                }
                spacing: 12
                Label { text: "Install safely"; color: "white"; font.pixelSize: 20; font.bold: true }
                Repeater {
                    model: root.steps
                    Label { required property string modelData; required property int index; text: (index < root.step ? "✓  " : index === root.step ? "●  " : "○  ") + modelData; color: index === root.step ? "#83ddca" : "#9babc1" }
                }
                Item { Layout.fillHeight: true }
                Label { Layout.fillWidth: true; text: "The base system installs offline. Network access and the local agent are optional."; color: "#8190a5"; wrapMode: Text.WordWrap }
            }
        }

        StackLayout {
            id: pages; currentIndex: root.step; Layout.fillWidth: true; Layout.fillHeight: true
            InstallerPage { heading: "Welcome"; body: "Try the live desktop for as long as you like. Installation never starts or erases a disk without a final review and exact confirmation." }
            Item {
                ColumnLayout {
                    anchors.centerIn: parent; width: Math.min(620, parent.width - 40); spacing: 16
                    Label { text: "Language and keyboard"; color: "white"; font.pixelSize: 34; font.bold: true }
                    Label { text: "Desktop language"; color: "#b9c7d8" }
                    ComboBox { id: locale; Layout.fillWidth: true; model: ["en_US.UTF-8", "en_GB.UTF-8", "tr_TR.UTF-8"] }
                    Label { text: "Keyboard layout"; color: "#b9c7d8" }
                    ComboBox { id: keymap; Layout.fillWidth: true; model: ["us", "uk", "trq"] }
                    TextField { Layout.fillWidth: true; placeholderText: "Type here to test your keyboard"; Accessible.name: "Keyboard test" }
                }
            }
            Item {
                ColumnLayout {
                    anchors.centerIn: parent; width: Math.min(620, parent.width - 40); spacing: 16
                    Label { text: "Connect or continue offline"; color: "white"; font.pixelSize: 34; font.bold: true }
                    Label { Layout.fillWidth: true; text: "Wi-Fi and Ethernet are optional for the verified base installation. Network access can be configured now or after installation."; color: "#b9c7d8"; wrapMode: Text.WordWrap }
                    RowLayout {
                        Button {
                            text: "Open network settings"
                            onClicked: Installer.openNetworkSettings()
                        }
                        Label {
                            text: "Offline installation is fully supported"
                            color: "#83ddca"
                        }
                    }
                }
            }
            Item {
                ColumnLayout {
                    anchors.centerIn: parent; width: Math.min(620, parent.width - 40); spacing: 18
                    Label { text: "Choose an installation disk"; color: "white"; font.pixelSize: 34; font.bold: true }
                    Label { Layout.fillWidth: true; text: "Only a qualified, unmounted internal disk can be selected. Live media and removable recovery disks are excluded."; color: "#b9c7d8"; wrapMode: Text.WordWrap }
                    Button { text: "Check this computer"; enabled: !Installer.busy; onClicked: Installer.observeTarget() }
                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 72
                        radius: 12
                        color: "#111c2a"
                        border.color: "#40536d"
                        Label {
                            anchors {
                                fill: parent
                                margins: 14
                            }
                            text: Installer.targetSummary.length ? Installer.targetSummary : "No target selected"
                            color: "white"
                            wrapMode: Text.WordWrap
                        }
                    }
                }
            }
            Item {
                ColumnLayout {
                    anchors.centerIn: parent; width: Math.min(600, parent.width - 40); spacing: 16
                    Label { text: "Create your account"; color: "white"; font.pixelSize: 32; font.bold: true }
                    Label { Layout.fillWidth: true; text: "This account owns your files and unlocks Blossom OS. There is no default password."; color: "#b9c7d8"; wrapMode: Text.WordWrap }
                    TextField { id: fullName; Layout.fillWidth: true; placeholderText: "Full name"; Accessible.name: "Full name" }
                    TextField { id: username; Layout.fillWidth: true; placeholderText: "Username"; Accessible.name: "Username" }
                    TextField { id: password; Layout.fillWidth: true; placeholderText: "Password"; echoMode: TextInput.Password; Accessible.name: "Password" }
                    TextField { id: passwordConfirmation; Layout.fillWidth: true; placeholderText: "Confirm password"; echoMode: TextInput.Password; Accessible.name: "Confirm password" }
                    CheckBox { id: administrator; text: "Allow this account to administer this computer"; checked: true }
                    ComboBox { id: agentMode; Layout.fillWidth: true; model: ["Agent off", "Agent available on demand"] }
                    Label { Layout.fillWidth: true; visible: !root.accountValid && (username.text.length > 0 || password.text.length > 0); text: "Use a lowercase username and matching password of at least 12 characters that does not contain the username."; color: "#ffb4a9"; wrapMode: Text.WordWrap; Accessible.role: Accessible.AlertMessage }
                }
            }
            Item {
                ColumnLayout {
                    anchors.centerIn: parent; width: Math.min(620, parent.width - 40); spacing: 16
                    Label { text: "Time zone"; color: "white"; font.pixelSize: 34; font.bold: true }
                    ComboBox { id: timezone; Layout.fillWidth: true; model: ["Europe/Istanbul", "Europe/London", "America/New_York", "America/Los_Angeles", "Asia/Tokyo"] }
                    Label { text: "Local preview: " + Qt.formatDateTime(new Date(), "dddd, d MMMM yyyy · HH:mm"); color: "#b9c7d8" }
                }
            }
            Item {
                ColumnLayout {
                    anchors.centerIn: parent; width: Math.min(640, parent.width - 40); spacing: 14
                    Label { text: "Review before erasing"; color: "white"; font.pixelSize: 32; font.bold: true }
                    Label { Layout.fillWidth: true; text: Installer.targetSummary; color: "#b9c7d8"; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; text: "Account: " + (username.text || "Not entered") + "\nLanguage: " + locale.currentText + "\nKeyboard: " + keymap.currentText + "\nTime zone: " + timezone.currentText + "\n" + agentMode.currentText; color: "#b9c7d8"; wrapMode: Text.WordWrap }
                    Label { Layout.fillWidth: true; text: "Type exactly: " + Installer.expectedConfirmation; color: "#ffb4a9"; wrapMode: Text.WordWrap }
                    TextField { id: eraseConfirmation; Layout.fillWidth: true; placeholderText: "Exact erase confirmation"; Accessible.name: "Exact erase confirmation" }
                }
            }
            Item {
                Connections { target: Installer; function onInstallationCompleted() { root.completed = true } }
                ColumnLayout {
                    anchors.centerIn: parent; width: Math.min(620, parent.width - 40); spacing: 18
                    BusyIndicator { Layout.alignment: Qt.AlignHCenter; running: Installer.busy }
                    Label { Layout.fillWidth: true; text: root.completed ? "Installation complete" : "Installing Blossom OS"; color: "white"; font.pixelSize: 34; font.bold: true; horizontalAlignment: Text.AlignHCenter }
                    Label { Layout.fillWidth: true; text: Installer.status; color: root.completed ? "#83ddca" : "#b9c7d8"; wrapMode: Text.WordWrap; horizontalAlignment: Text.AlignHCenter; Accessible.role: Accessible.AlertMessage }
                }
            }
        }
    }

    footer: Rectangle {
        height: 82; color: "#111c2a"
        RowLayout {
            anchors {
                fill: parent
                leftMargin: 28
                rightMargin: 28
            }
            spacing: 12
            Button { text: "Quit"; enabled: !Installer.busy; onClicked: root.close() }
            Item { Layout.fillWidth: true }
            Button { text: "Back"; enabled: !Installer.busy && root.step > 0 && root.step < root.steps.length - 1; onClicked: root.step-- }
            Button {
                text: root.step === root.steps.length - 2 ? "Erase and install" : root.step === root.steps.length - 1 ? (root.completed ? "Restart later" : "Installing…") : "Continue"
                enabled: !Installer.busy && (root.step !== 3 || Installer.targetSummary.length > 0) && (root.step !== 4 || root.accountValid) && (root.step !== 6 || eraseConfirmation.text === Installer.expectedConfirmation)
                onClicked: {
                    if (root.step === 6) {
                        root.step = 7
                        Installer.install(fullName.text, username.text, password.text, passwordConfirmation.text,
                                          locale.currentText, timezone.currentText, keymap.currentText, administrator.checked,
                                          agentMode.currentIndex === 0 ? "off" : "on_demand", eraseConfirmation.text)
                        password.text = ""
                        passwordConfirmation.text = ""
                    } else if (root.step < root.steps.length - 1) root.step++
                }
            }
        }
    }

    component InstallerPage: Item {
        required property string heading
        required property string body
        ColumnLayout {
            anchors.centerIn: parent; width: Math.min(620, parent.width - 40); spacing: 18
            Label { Layout.fillWidth: true; text: heading; color: "white"; font.pixelSize: 34; font.bold: true; wrapMode: Text.WordWrap }
            Label { Layout.fillWidth: true; text: body; color: "#b9c7d8"; font.pixelSize: 17; wrapMode: Text.WordWrap }
        }
    }
}
