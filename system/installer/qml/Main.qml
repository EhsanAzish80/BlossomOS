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
        anchors { fill: parent; margins: 28 }; spacing: 28
        Rectangle {
            Layout.preferredWidth: 220; Layout.fillHeight: true; radius: 20; color: "#101a27"
            ColumnLayout {
                anchors { fill: parent; margins: 22 }; spacing: 12
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
            InstallerPage { heading: "Language and keyboard"; body: "Choose the language used by the installer and your desktop, then verify the keyboard layout in a test field." }
            InstallerPage { heading: "Connect or continue offline"; body: "Wi-Fi and Ethernet are optional for the verified base installation. Unsupported adapters will show recovery guidance instead of blocking you." }
            InstallerPage { heading: "Choose an installation disk"; body: "Internal disks are shown with model, capacity and current partitions. Live media and removable recovery disks cannot be selected." }
            Item {
                ColumnLayout {
                    anchors.centerIn: parent; width: Math.min(600, parent.width - 40); spacing: 16
                    Label { text: "Create your account"; color: "white"; font.pixelSize: 32; font.bold: true }
                    Label { Layout.fillWidth: true; text: "This account owns your files and unlocks Blossom OS. There is no default password."; color: "#b9c7d8"; wrapMode: Text.WordWrap }
                    TextField { Layout.fillWidth: true; placeholderText: "Full name"; Accessible.name: "Full name" }
                    TextField { Layout.fillWidth: true; placeholderText: "Username"; Accessible.name: "Username" }
                    TextField { Layout.fillWidth: true; placeholderText: "Password"; echoMode: TextInput.Password; Accessible.name: "Password" }
                    TextField { Layout.fillWidth: true; placeholderText: "Confirm password"; echoMode: TextInput.Password; Accessible.name: "Confirm password" }
                    CheckBox { text: "Allow this account to administer this computer"; checked: true }
                }
            }
            InstallerPage { heading: "Time zone"; body: "Select your location and verify the local date and time before installation." }
            InstallerPage { heading: "Review before erasing"; body: "Review language, network, target disk, account, time zone and agent defaults. The exact target identity remains visible during confirmation." }
            InstallerPage { heading: "Installing Blossom OS"; body: "A readable progress view will show partitioning, verified file installation, account creation and boot setup. Failure preserves a support log without passwords." }
        }
    }

    footer: Rectangle {
        height: 82; color: "#111c2a"
        RowLayout {
            anchors { fill: parent; leftMargin: 28; rightMargin: 28 }; spacing: 12
            Button { text: "Quit"; onClicked: root.close() }
            Item { Layout.fillWidth: true }
            Button { text: "Back"; enabled: root.step > 0 && root.step < root.steps.length - 1; onClicked: root.step-- }
            Button { text: root.step === root.steps.length - 2 ? "Confirm installation" : root.step === root.steps.length - 1 ? "Finish" : "Continue"; onClicked: { if (root.step < root.steps.length - 1) root.step++ } }
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
