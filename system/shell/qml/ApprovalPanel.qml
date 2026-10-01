import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import Blossom.Shell

Window {
    id: approvalWindow
    property var broker: BlossomBroker
    visible: broker.state === "waiting" || broker.state === "submitting" || broker.state === "cancelling"
    x: Math.max(0, Math.round(((screen ? screen.width : 800) - width) / 2))
    y: Math.max(52, Math.round(((screen ? screen.height : 600) - height) / 2))
    width: Math.max(640, Math.min(1040, (screen ? screen.width : 800) - 120))
    height: Math.max(440, Math.min(656, (screen ? screen.height : 600) - 144))
    flags: Qt.Dialog | Qt.FramelessWindowHint |
           (broker.state === "waiting" ? Qt.WindowStaysOnTopHint : 0)
    modality: broker.state === "waiting" ? Qt.ApplicationModal : Qt.NonModal
    color: "#e611151c"
    title: "Blossom OS approval"

    onActiveChanged: {
        if (active && denyButton.enabled) {
            denyButton.forceActiveFocus(Qt.ActiveWindowFocusReason);
        }
    }

    onClosing: close => {
        if (broker.state === "waiting") {
            broker.cancelPending();
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
        enabled: approvalWindow.visible && broker.state === "waiting"
        autoRepeat: false
        onActivated: broker.cancelPending()
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
            if (broker.state === "waiting") {
                broker.cancelPending();
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
                    text: "🔒  Blossom approval · drawn by the system broker"
                    textFormat: Text.PlainText
                    color: "#dbe5f5"
                    font.pixelSize: 14
                    font.bold: true
                    wrapMode: Text.WrapAnywhere
                    Accessible.role: Accessible.StaticText
                    Accessible.name: "System broker approval surface"
                    Accessible.description: text
                    Accessible.ignored: false
                }

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

                ScrollView {
                    id: previewScroll
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true

                    ColumnLayout {
                        width: previewScroll.availableWidth
                        spacing: 12

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16

                            Frame {
                                Layout.fillWidth: true
                                Layout.preferredWidth: 1

                                ColumnLayout {
                                    anchors.fill: parent

                                    Label {
                                        text: "You asked"
                                        color: "#8ee6d7"
                                        font.bold: true
                                    }

                                    Label {
                                        Layout.fillWidth: true
                                        text: broker.preview.user_request ?? "Fixed system request"
                                        textFormat: Text.PlainText
                                        wrapMode: Text.WrapAnywhere
                                        color: "#f4f7fb"
                                        Accessible.role: Accessible.StaticText
                                        Accessible.name: "Original request"
                                        Accessible.description: text
                                        Accessible.ignored: false
                                    }
                                }
                            }

                            Frame {
                                Layout.fillWidth: true
                                Layout.preferredWidth: 1

                                ColumnLayout {
                                    anchors.fill: parent

                                    Label {
                                        text: "Blossom will"
                                        color: "#ffcc80"
                                        font.bold: true
                                    }

                                    Label {
                                        Layout.fillWidth: true
                                        text: broker.preview.proposal_source === "parsed_directly"
                                            ? "Parsed directly from your request"
                                            : broker.preview.proposal_source === "model_proposed"
                                                ? "Proposed by the local model"
                                                : "Prepared by trusted system code"
                                        wrapMode: Text.WordWrap
                                        color: broker.preview.proposal_source === "model_proposed" ? "#ffcc80" : "#8ee6d7"
                                        Accessible.role: Accessible.StaticText
                                        Accessible.name: "Proposal source"
                                        Accessible.description: text
                                        Accessible.ignored: false
                                    }

                                    Label {
                                        Layout.fillWidth: true
                                        text: broker.preview.destination ?? "No file effect"
                                        textFormat: Text.PlainText
                                        wrapMode: Text.WrapAnywhere
                                        color: "#f4f7fb"
                                        font.family: "monospace"
                                        Accessible.role: Accessible.StaticText
                                        Accessible.name: "Full destination"
                                        Accessible.description: text
                                        Accessible.ignored: false
                                    }

                                    Label {
                                        Layout.fillWidth: true
                                        text: "Destination folder: " + approvalWindow.destinationFolder
                                        textFormat: Text.PlainText
                                        wrapMode: Text.WrapAnywhere
                                        color: "#b9c7d8"
                                        Accessible.role: Accessible.StaticText
                                        Accessible.name: "Destination folder"
                                        Accessible.description: approvalWindow.destinationFolder
                                        Accessible.ignored: false
                                    }

                                    Label {
                                        Layout.fillWidth: true
                                        text: String(broker.preview.content_bytes ?? 0) + " UTF-8 bytes"
                                        color: "#b9c7d8"
                                        Accessible.role: Accessible.StaticText
                                        Accessible.name: "Content byte length"
                                        Accessible.description: text
                                        Accessible.ignored: false
                                    }
                                }
                            }
                        }

                        Label {
                            visible: broker.preview.content !== undefined
                            text: "Complete content"
                            color: "#ffcc80"
                            font.bold: true
                        }

                        ScrollView {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 160
                            visible: broker.preview.content !== undefined
                            clip: true

                            TextArea {
                                text: broker.preview.content ?? ""
                                readOnly: true
                                selectByMouse: true
                                textFormat: TextEdit.PlainText
                                wrapMode: TextEdit.WrapAnywhere
                                font.family: "monospace"
                                color: "#f4f7fb"
                                background: Rectangle {
                                    color: "#111720"
                                    border.color: "#3b4658"
                                    radius: 8
                                }
                                Accessible.role: Accessible.StaticText
                                Accessible.name: "Complete proposed content"
                                Accessible.description: text
                                Accessible.ignored: false
                            }
                        }

                        GridLayout {
                            Layout.fillWidth: true
                            columns: 2
                            columnSpacing: 20
                            rowSpacing: 7

                            SecurityField { label: "Operation"; value: broker.preview.operation ?? "" }
                            SecurityField { label: "Purpose"; value: broker.preview.purpose ?? "" }
                            SecurityField { label: "Executable"; value: broker.preview.executable ?? "" }
                            SecurityField { label: "Arguments"; value: (broker.preview.arguments ?? []).join(" ") }
                            SecurityField { label: "Capability"; value: broker.preview.capability ?? "" }
                            SecurityField { label: "Resource scope"; value: broker.preview.resource_scope ?? "" }
                            SecurityField { label: "Filesystem"; value: broker.preview.filesystem ?? "" }
                            SecurityField { label: "Network"; value: broker.preview.network ?? "" }
                            SecurityField { label: "Privilege"; value: broker.preview.privilege ?? "" }
                            SecurityField { label: "Expected side effects"; value: broker.preview.expected_side_effects ?? "" }
                            SecurityField { label: "Approval"; value: broker.preview.approval ?? "" }
                            SecurityField { label: "Expires at (ms)"; value: String(broker.preview.expires_at_ms ?? "") }
                            SecurityField { label: "Request ID"; value: broker.preview.request_id ?? "" }
                            SecurityField { label: "Preview SHA-256"; value: broker.preview.preview_sha256 ?? "" }
                        }
                    }
                }

                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 12

                    Button {
                        id: denyButton
                        text: "Deny"
                        enabled: broker.state === "waiting"
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
                        onClicked: broker.deny()
                    }

                    Button {
                        id: approveButton
                        text: "Approve once"
                        enabled: broker.state === "waiting"
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
                        onClicked: broker.approveOnce()
                    }
                }
            }
        }
    }

    readonly property string destinationFolder: {
        const destination = String(broker.preview.destination ?? "")
        const separator = destination.lastIndexOf("/")
        return separator > 0 ? destination.slice(0, separator) : destination
    }
}
