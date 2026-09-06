import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import Blossom.Shell

Window {
    id: approvalWindow
    visible: BlossomBroker.state === "waiting" || BlossomBroker.state === "submitting" || BlossomBroker.state === "cancelling"
    x: Math.max(0, Math.round(((screen ? screen.width : 800) - width) / 2))
    y: Math.max(52, Math.round(((screen ? screen.height : 600) - height) / 2))
    width: Math.max(640, Math.min(1040, (screen ? screen.width : 800) - 120))
    height: Math.max(440, Math.min(656, (screen ? screen.height : 600) - 144))
    flags: Qt.Dialog | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    modality: Qt.ApplicationModal
    color: "#e611151c"
    title: "Blossom OS approval"

    onClosing: close => {
        if (BlossomBroker.state === "waiting") {
            BlossomBroker.cancelPending();
        }
        close.accepted = false;
    }

    onVisibleChanged: {
        if (visible) {
            requestActivate();
            denyButton.forceActiveFocus(Qt.ActiveWindowFocusReason);
        }
    }

    Shortcut {
        sequence: "Escape"
        context: Qt.WindowShortcut
        enabled: approvalWindow.visible && BlossomBroker.state === "waiting"
        autoRepeat: false
        onActivated: BlossomBroker.cancelPending()
    }

    FocusScope {
        id: approvalFocus
        anchors.fill: parent
        focus: approvalWindow.visible
        Accessible.role: Accessible.Dialog
        Accessible.name: "Approval required"
        Accessible.description: "Review the fixed security fields, then deny or approve this request once."
        Accessible.ignored: false

        Keys.onEscapePressed: event => {
            if (BlossomBroker.state === "waiting") {
                BlossomBroker.cancelPending();
            }
            event.accepted = true;
        }

        Rectangle {
            anchors.fill: parent
            radius: 18
            color: "#171d27"
            border.color: "#3b4658"
            border.width: 1

            ColumnLayout {
                anchors {
                    fill: parent
                    margins: 28
                }
                spacing: 10

                Label {
                    Layout.fillWidth: true
                    color: "#f4f7fb"
                    font.pixelSize: 24
                    font.bold: true
                    text: "Approval required"
                    Accessible.role: Accessible.Heading
                    Accessible.name: text
                }

                Label {
                    Layout.fillWidth: true
                    color: "#ffcc80"
                    wrapMode: Text.WordWrap
                    text: "Review every fixed security field. This request can be approved once or denied."
                    Accessible.role: Accessible.StaticText
                    Accessible.name: text
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    columnSpacing: 20
                    rowSpacing: 7

                    SecurityField { label: "Operation"; value: BlossomBroker.preview.operation ?? "" }
                    SecurityField { label: "Purpose"; value: BlossomBroker.preview.purpose ?? "" }
                    SecurityField { label: "Executable"; value: BlossomBroker.preview.executable ?? "" }
                    SecurityField { label: "Arguments"; value: (BlossomBroker.preview.arguments ?? []).join(" ") }
                    SecurityField { label: "Capability"; value: BlossomBroker.preview.capability ?? "" }
                    SecurityField { label: "Resource scope"; value: BlossomBroker.preview.resource_scope ?? "" }
                    SecurityField { label: "Filesystem"; value: BlossomBroker.preview.filesystem ?? "" }
                    SecurityField { label: "Network"; value: BlossomBroker.preview.network ?? "" }
                    SecurityField { label: "Privilege"; value: BlossomBroker.preview.privilege ?? "" }
                    SecurityField { label: "Expected side effects"; value: BlossomBroker.preview.expected_side_effects ?? "" }
                    SecurityField { label: "Approval"; value: BlossomBroker.preview.approval ?? "" }
                    SecurityField { label: "Expires at (ms)"; value: String(BlossomBroker.preview.expires_at_ms ?? "") }
                    SecurityField { label: "Request ID"; value: BlossomBroker.preview.request_id ?? "" }
                    SecurityField { label: "Preview SHA-256"; value: BlossomBroker.preview.preview_sha256 ?? "" }
                }

                Item { Layout.fillHeight: true }

                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 12

                    Button {
                        id: denyButton
                        text: "Deny"
                        enabled: BlossomBroker.state === "waiting"
                        activeFocusOnTab: true
                        KeyNavigation.tab: approveButton
                        KeyNavigation.backtab: approveButton
                        Accessible.name: text
                        Accessible.description: "Deny this request without starting execution."
                        Accessible.role: Accessible.Button
                        Accessible.ignored: false
                        Accessible.defaultButton: true
                        Accessible.onPressAction: {
                            if (enabled) {
                                clicked();
                            }
                        }
                        onClicked: BlossomBroker.deny()
                    }

                    Button {
                        id: approveButton
                        text: "Approve once"
                        enabled: BlossomBroker.state === "waiting"
                        activeFocusOnTab: true
                        KeyNavigation.tab: denyButton
                        KeyNavigation.backtab: denyButton
                        Accessible.name: text
                        Accessible.description: "Approve only this exact request for one execution."
                        Accessible.role: Accessible.Button
                        Accessible.ignored: false
                        Accessible.onPressAction: {
                            if (enabled) {
                                clicked();
                            }
                        }
                        onClicked: BlossomBroker.approveOnce()
                    }
                }
            }
        }
    }
}
