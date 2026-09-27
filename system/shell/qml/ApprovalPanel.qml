import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import Blossom.Shell
import "Petal"

Window {
    id: approvalWindow
    visible: BlossomBroker.state === "waiting" || BlossomBroker.state === "submitting" || BlossomBroker.state === "cancelling"
    x: Math.max(0, Math.round(((screen ? screen.width : 800) - width) / 2))
    y: Math.max(52, Math.round(((screen ? screen.height : 600) - height) / 2))
    width: Math.max(640, Math.min(960, (screen ? screen.width : 800) - 120))
    height: Math.max(440, Math.min(600, (screen ? screen.height : 600) - 144))
    flags: Qt.Dialog | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    modality: Qt.ApplicationModal
    color: "transparent"
    title: "Blossom OS approval"

    property double now: Date.now()
    readonly property double expiresAt: Number(BlossomBroker.preview.expires_at_ms ?? 0)
    readonly property int secondsLeft: Math.max(0, Math.ceil((expiresAt - now) / 1000))
    readonly property string purposeQuestion: {
        const purpose = String(BlossomBroker.preview.purpose ?? "");
        return purpose.length > 0 ? purpose.charAt(0).toUpperCase() + purpose.slice(1) + "?" : "Review this request";
    }

    Timer {
        interval: 1000
        repeat: true
        running: approvalWindow.visible
        triggeredOnStart: true
        onTriggered: approvalWindow.now = Date.now()
    }

    onActiveChanged: {
        if (active && denyButton.enabled) {
            denyButton.forceActiveFocus(Qt.ActiveWindowFocusReason);
        }
    }

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

        // Approval surfaces are deliberately static: no motion, one amber cue.
        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusDock
            color: Theme.surface
            border.color: Theme.lineStrong
            border.width: 1

            ColumnLayout {
                anchors {
                    fill: parent
                    margins: Theme.s6
                }
                spacing: Theme.s3

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.s2

                    Rectangle {
                        implicitWidth: headingRow.implicitWidth + 22
                        implicitHeight: 28
                        radius: Theme.radiusChip
                        color: Theme.decisionTint

                        Row {
                            id: headingRow
                            anchors.centerIn: parent
                            spacing: 6

                            Image {
                                anchors.verticalCenter: parent.verticalCenter
                                width: 15
                                height: 15
                                sourceSize.width: 30
                                sourceSize.height: 30
                                source: "Petal/icons/shield.svg"
                            }

                            Label {
                                anchors.verticalCenter: parent.verticalCenter
                                color: Theme.decision
                                font.family: Theme.sans
                                font.pixelSize: Theme.caption
                                font.weight: Font.DemiBold
                                text: "Approval required"
                                Accessible.role: Accessible.Heading
                                Accessible.name: text
                            }
                        }
                    }

                    Item { Layout.fillWidth: true }

                    Image {
                        Layout.preferredWidth: 16
                        Layout.preferredHeight: 16
                        sourceSize.width: 32
                        sourceSize.height: 32
                        source: "Petal/icons/clock.svg"
                    }

                    Label {
                        color: approvalWindow.secondsLeft <= 10 ? Theme.decision : Theme.textSecondary
                        font.family: Theme.sans
                        font.pixelSize: Theme.label
                        text: "Expires in " + Math.floor(approvalWindow.secondsLeft / 60) + ":" + String(approvalWindow.secondsLeft % 60).padStart(2, "0")
                        Accessible.ignored: true
                    }
                }

                Label {
                    Layout.fillWidth: true
                    color: Theme.text
                    font.family: Theme.sans
                    font.pixelSize: Theme.headline
                    font.weight: Font.DemiBold
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    textFormat: Text.PlainText
                    text: approvalWindow.purposeQuestion
                    Accessible.ignored: true
                }

                // What the request can touch.
                GridLayout {
                    Layout.fillWidth: true
                    columns: 4
                    columnSpacing: Theme.s2

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        implicitHeight: filesystemField.implicitHeight + 24
                        radius: Theme.radiusTile
                        color: Theme.raised
                        SecurityField { id: filesystemField; anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 12 } label: "Filesystem"; value: BlossomBroker.preview.filesystem ?? "" }
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        implicitHeight: filesystemField.implicitHeight + 24
                        radius: Theme.radiusTile
                        color: Theme.raised
                        SecurityField { anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 12 } label: "Network"; value: BlossomBroker.preview.network ?? "" }
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        implicitHeight: filesystemField.implicitHeight + 24
                        radius: Theme.radiusTile
                        color: Theme.raised
                        SecurityField { anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 12 } label: "Privilege"; value: BlossomBroker.preview.privilege ?? "" }
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        implicitHeight: filesystemField.implicitHeight + 24
                        radius: Theme.radiusTile
                        color: Theme.raised
                        SecurityField { anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; margins: 12 } label: "Expected side effects"; value: BlossomBroker.preview.expected_side_effects ?? "" }
                    }
                }

                // Exactly what runs, then the evidence that binds it.
                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumHeight: evidence.implicitHeight + 32
                    radius: Theme.radiusTile
                    color: Theme.ink0
                    border.color: Theme.line
                    border.width: 1

                    GridLayout {
                        id: evidence
                        anchors {
                            left: parent.left
                            right: parent.right
                            top: parent.top
                            margins: Theme.s4
                        }
                        columns: 3
                        columnSpacing: Theme.s5
                        rowSpacing: Theme.s3

                        SecurityField { Layout.columnSpan: 2; Layout.preferredWidth: 2; mono: true; label: "Executable"; value: BlossomBroker.preview.executable ?? "" }
                        SecurityField { Layout.preferredWidth: 1; mono: true; label: "Arguments"; value: (BlossomBroker.preview.arguments ?? []).join(" ") }
                        SecurityField { Layout.preferredWidth: 1; label: "Purpose"; value: BlossomBroker.preview.purpose ?? "" }
                        SecurityField { Layout.preferredWidth: 1; mono: true; label: "Operation"; value: BlossomBroker.preview.operation ?? "" }
                        SecurityField { Layout.preferredWidth: 1; mono: true; label: "Capability"; value: BlossomBroker.preview.capability ?? "" }
                        SecurityField { Layout.preferredWidth: 1; label: "Resource scope"; value: BlossomBroker.preview.resource_scope ?? "" }
                        SecurityField { Layout.preferredWidth: 1; label: "Approval"; value: BlossomBroker.preview.approval ?? "" }
                        SecurityField { Layout.preferredWidth: 1; mono: true; label: "Request ID"; value: BlossomBroker.preview.request_id ?? "" }
                        SecurityField {
                            Layout.preferredWidth: 1
                            mono: true
                            label: "Expires at (ms)"
                            value: String(BlossomBroker.preview.expires_at_ms ?? "")
                            shown: approvalWindow.expiresAt > 0 ? Qt.formatTime(new Date(approvalWindow.expiresAt), "HH:mm:ss") + "  (" + value + ")" : value
                        }
                        SecurityField {
                            Layout.columnSpan: 2
                            Layout.preferredWidth: 2
                            mono: true
                            wrapMode: Text.WrapAnywhere
                            maximumLineCount: 2
                            label: "Preview SHA-256"
                            value: BlossomBroker.preview.preview_sha256 ?? ""
                            shown: value.replace(/(.{4})(?=.)/g, "$1 ")
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.s3

                    Label {
                        Layout.fillWidth: true
                        color: Theme.textTertiary
                        font.family: Theme.sans
                        font.pixelSize: Theme.caption
                        text: "Esc cancels · Tab switches"
                        Accessible.ignored: true
                    }

                    BlossomButton {
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

                    BlossomButton {
                        id: approveButton
                        primary: true
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
