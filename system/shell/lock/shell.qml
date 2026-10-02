import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pam
import Quickshell.Wayland
import "Petal"

ShellRoot {
    id: root
    property alias pamContext: pam

    Timer {
        id: restartPam
        interval: 150
        repeat: false
        onTriggered: {
            if (!pam.start())
                lock.feedback = "Authentication could not start. The screen remains locked."
        }
    }

    Timer {
        id: retryDelay
        interval: 1000
        repeat: true
        onTriggered: {
            lock.retrySecondsRemaining -= 1
            if (lock.retrySecondsRemaining <= 0) {
                stop()
                lock.feedback = "You can try again."
                restartPam.restart()
            } else {
                lock.feedback = "Incorrect password. Try again in "
                    + lock.retrySecondsRemaining + " seconds."
            }
        }
    }

    PamContext {
        id: pam
        config: "blossom-lock"
        user: "blossom"
        Component.onCompleted: {
            if (!start())
                lock.feedback = "Authentication could not start. The screen remains locked."
        }
        onPamMessage: {
            if (responseRequired) {
                lock.feedback = ""
                if (lock.activePasswordField !== null)
                    lock.activePasswordField.forceActiveFocus(Qt.ActiveWindowFocusReason)
            } else if (message.length > 0) {
                lock.feedback = message
            }
        }
        onCompleted: result => {
            if (lock.activePasswordField !== null)
                lock.activePasswordField.text = ""
            if (result === PamResult.Success) {
                lock.feedback = ""
                lock.locked = false
                stopSelf.running = true
            } else {
                lock.beginFailureDelay()
            }
        }
        onError: error => {
            lock.beginFailureDelay()
        }
    }

    Process {
        id: stopSelf
        command: ["/usr/bin/systemctl", "--user", "stop", "blossom-lock.service"]
    }

    WlSessionLock {
        id: lock
        locked: true
        property string feedback: ""
        property var activePasswordField: null
        property int failedAttempts: 0
        property int retrySecondsRemaining: 0

        function beginFailureDelay(): void {
            if (retryDelay.running)
                return
            failedAttempts += 1
            retrySecondsRemaining = Math.min(30, Math.pow(2, Math.min(failedAttempts - 1, 5)))
            feedback = "Incorrect password. Try again in "
                + retrySecondsRemaining + " seconds."
            retryDelay.start()
        }

        function authenticate(response: string): void {
            if (!root.pamContext.active || !root.pamContext.responseRequired || response.length === 0)
                return
            feedback = "Checking…"
            root.pamContext.respond(response)
        }

        WlSessionLockSurface {
            id: lockSurface
            color: Theme.ink0
            Rectangle {
                anchors.fill: parent
                color: Theme.ink0
                ColumnLayout {
                    anchors.centerIn: parent
                    width: Math.min(440, parent.width - 48)
                    spacing: Theme.s4
                    Image {
                        Layout.alignment: Qt.AlignHCenter
                        source: Qt.resolvedUrl("Petal/icons/lock.svg")
                        sourceSize.width: 48
                        sourceSize.height: 48
                        Accessible.ignored: true
                    }
                    Label {
                        Layout.fillWidth: true
                        text: Qt.formatDateTime(new Date(), "HH:mm")
                        color: Theme.text
                        font.family: Theme.sans
                        font.pixelSize: Theme.display
                        font.bold: true
                        horizontalAlignment: Text.AlignHCenter
                    }
                    Label {
                        Layout.fillWidth: true
                        text: "Blossom OS is locked"
                        color: Theme.textSecondary
                        font.family: Theme.sans
                        font.pixelSize: Theme.bodyLarge
                        horizontalAlignment: Text.AlignHCenter
                    }
                    TextField {
                        id: passwordField
                        Layout.fillWidth: true
                        Layout.preferredHeight: 48
                        visible: lockSurface.screen === Quickshell.screens[0]
                        enabled: root.pamContext.responseRequired && !retryDelay.running
                        placeholderText: "Password"
                        echoMode: showPassword.checked ? TextInput.Normal : TextInput.Password
                        passwordMaskDelay: 0
                        Accessible.name: "Password"
                        Accessible.description: "Enter your password to unlock Blossom OS"
                        onAccepted: lock.authenticate(text)
                        Component.onCompleted: {
                            if (visible) {
                                lock.activePasswordField = passwordField
                                passwordField.forceActiveFocus(Qt.ActiveWindowFocusReason)
                            }
                        }
                    }
                    CheckBox {
                        id: showPassword
                        visible: lockSurface.screen === Quickshell.screens[0]
                        text: "Show password"
                        checked: false
                        font.family: Theme.sans
                        font.pixelSize: Theme.label
                        Accessible.name: text
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: lock.feedback.length > 0
                        text: lock.feedback
                        color: Theme.danger
                        font.family: Theme.sans
                        font.pixelSize: Theme.body
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        Accessible.role: Accessible.AlertMessage
                        Accessible.name: text
                    }
                    Button {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Theme.controlHeight
                        visible: lockSurface.screen === Quickshell.screens[0]
                        text: retryDelay.running ? "Wait " + lock.retrySecondsRemaining + "s"
                            : root.pamContext.responseRequired ? "Unlock" : "Checking…"
                        enabled: passwordField.text.length > 0
                            && root.pamContext.responseRequired && !retryDelay.running
                        onClicked: lock.authenticate(passwordField.text)
                        contentItem: Text {
                            text: parent.text
                            color: parent.enabled ? Theme.blossomForeground : Theme.textDisabled
                            font.family: Theme.sans
                            font.pixelSize: Theme.label
                            font.weight: Font.DemiBold
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        background: Rectangle {
                            radius: Theme.radiusControl
                            color: parent.down ? Theme.blossomPressed
                                : parent.hovered ? Theme.blossomHover : Theme.blossom
                        }
                    }
                }
            }
        }
    }
}
