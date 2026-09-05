import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Blossom.Shell

PanelWindow {
    id: approvalWindow
    visible: BlossomBroker.state === "waiting" || BlossomBroker.state === "submitting" || BlossomBroker.state === "cancelling"
    focusable: true
    color: "#e611151c"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    anchors {
        top: true
        bottom: true
        left: true
        right: true
    }
    margins {
        top: 72
        bottom: 72
        left: 120
        right: 120
    }

    onVisibleChanged: {
        if (visible) {
            requestActivate();
            approvalFocus.forceActiveFocus(Qt.ActiveWindowFocusReason);
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
                }

                Label {
                    Layout.fillWidth: true
                    color: "#ffcc80"
                    wrapMode: Text.WordWrap
                    text: "Review every fixed security field. This request can be approved once or denied."
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
                        text: "Deny"
                        enabled: BlossomBroker.state === "waiting"
                        onClicked: BlossomBroker.deny()
                    }

                    Button {
                        text: "Approve once"
                        enabled: BlossomBroker.state === "waiting"
                        onClicked: BlossomBroker.approveOnce()
                    }
                }
            }
        }
    }
}
