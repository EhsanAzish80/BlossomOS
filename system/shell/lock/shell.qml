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
        id: retryPamAfterSystemError
        interval: 3000
        repeat: false
        onTriggered: restartPam.restart()
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

    Process {
        id: retryState
        property string action: ""
        stdout: StdioCollector {
            onStreamFinished: root.handleRetryState(retryState.action, text)
        }
        onExited: (exitCode, exitStatus) => {
            if (exitCode !== 0) {
                retryDelay.stop()
                lock.feedback = "Can't enforce the password retry delay. The screen remains locked."
            }
        }
    }

    Process {
        id: resetRetryState
        command: ["/usr/lib/blossom-os/blossom-lock-delay", "reset"]
        onExited: (exitCode, exitStatus) => {
            if (exitCode !== 0) {
                lock.feedback = "Can't clear the password retry state. The screen remains locked."
                return
            }
            lock.feedback = ""
            lock.locked = false
            stopSelf.running = true
        }
    }

    PamContext {
        id: pam
        config: "blossom-lock"
        user: "blossom"
        Component.onCompleted: root.readRetryState()
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
                resetRetryState.running = true
            } else if (result === PamResult.Failed || result === PamResult.MaxTries) {
                root.recordAuthenticationFailure()
            }
        }
        onError: error => {
            lock.feedback = "Can't check the password right now. The screen remains locked."
            retryPamAfterSystemError.restart()
        }
    }

    Process {
        id: stopSelf
        command: ["/usr/bin/systemctl", "--user", "stop", "blossom-lock.service"]
    }

    Process {
        id: suspendSystem
        command: ["/usr/bin/systemctl", "suspend"]
    }

    Process {
        id: restartSystem
        command: ["/usr/bin/systemctl", "reboot"]
    }

    Process {
        id: powerOffSystem
        command: ["/usr/bin/systemctl", "poweroff"]
    }

    function readRetryState(): void {
        retryState.action = "status"
        retryState.exec(["/usr/lib/blossom-os/blossom-lock-delay", "status"])
    }

    function recordAuthenticationFailure(): void {
        retryState.action = "failure"
        retryState.exec(["/usr/lib/blossom-os/blossom-lock-delay", "failure"])
    }

    function handleRetryState(action: string, output: string): void {
        const fields = output.trim().split(/\s+/)
        if (fields.length !== 2) {
            lock.feedback = "Can't enforce the password retry delay. The screen remains locked."
            return
        }
        const attempts = Number(fields[0])
        const remaining = Number(fields[1])
        if (!Number.isInteger(attempts) || attempts < 0
                || !Number.isInteger(remaining) || remaining < 0 || remaining > 30) {
            lock.feedback = "Can't enforce the password retry delay. The screen remains locked."
            return
        }
        lock.failedAttempts = attempts
        lock.retrySecondsRemaining = remaining
        if (remaining > 0) {
            lock.feedback = "Incorrect password. Try again in " + remaining + " seconds."
            retryDelay.start()
        } else if (!pam.start()) {
            lock.feedback = "Authentication could not start. The screen remains locked."
        }
    }

    WlSessionLock {
        id: lock
        locked: true
        property string feedback: ""
        property var activePasswordField: null
        property int failedAttempts: 0
        property int retrySecondsRemaining: 0

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
                id: lockBackground
                anchors.fill: parent
                color: Theme.ink0
                property bool powerMenuOpen: false

                Button {
                    id: powerMenuButton
                    anchors { left: parent.left; top: parent.top; margins: Theme.s5 }
                    z: 3
                    visible: lockSurface.screen === Quickshell.screens[0]
                    text: "Power"
                    Accessible.name: "Power options"
                    onClicked: {
                        lockBackground.powerMenuOpen = !lockBackground.powerMenuOpen
                        if (lockBackground.powerMenuOpen)
                            sleepButton.forceActiveFocus(Qt.PopupFocusReason)
                    }
                    Keys.onEscapePressed: lockBackground.powerMenuOpen = false
                    contentItem: Text {
                        text: powerMenuButton.text
                        color: Theme.text
                        font.family: Theme.sans
                        font.pixelSize: Theme.label
                        font.weight: Font.DemiBold
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                    background: Rectangle {
                        radius: Theme.radiusControl
                        color: powerMenuButton.down ? Theme.pressed
                            : powerMenuButton.hovered ? Theme.hover : Theme.panelFill
                        border { color: Theme.lineStrong; width: 1 }
                    }
                }

                Rectangle {
                    anchors { left: powerMenuButton.left; top: powerMenuButton.bottom; topMargin: Theme.s2 }
                    z: 4
                    width: 180
                    height: powerMenuColumn.implicitHeight + Theme.s3 * 2
                    visible: lockBackground.powerMenuOpen
                    radius: Theme.radiusControl
                    color: Theme.panelFill
                    border { color: Theme.lineStrong; width: 1 }
                    ColumnLayout {
                        id: powerMenuColumn
                        anchors { fill: parent; margins: Theme.s3 }
                        spacing: Theme.s1
                        Button {
                            id: sleepButton
                            Layout.fillWidth: true
                            text: "Sleep"
                            Accessible.name: text
                            Keys.onEscapePressed: lockBackground.powerMenuOpen = false
                            onClicked: {
                                lockBackground.powerMenuOpen = false
                                suspendSystem.running = true
                            }
                            contentItem: Text {
                                text: sleepButton.text
                                color: Theme.text
                                font.family: Theme.sans
                                font.pixelSize: Theme.label
                                horizontalAlignment: Text.AlignLeft
                                verticalAlignment: Text.AlignVCenter
                            }
                            background: Rectangle {
                                radius: Theme.radiusChip
                                color: sleepButton.down ? Theme.pressed
                                    : sleepButton.hovered ? Theme.hover : "transparent"
                            }
                        }
                        Button {
                            id: restartButton
                            Layout.fillWidth: true
                            text: "Restart"
                            Accessible.name: text
                            Keys.onEscapePressed: lockBackground.powerMenuOpen = false
                            onClicked: restartSystem.running = true
                            contentItem: Text {
                                text: restartButton.text
                                color: Theme.text
                                font.family: Theme.sans
                                font.pixelSize: Theme.label
                                horizontalAlignment: Text.AlignLeft
                                verticalAlignment: Text.AlignVCenter
                            }
                            background: Rectangle {
                                radius: Theme.radiusChip
                                color: restartButton.down ? Theme.pressed
                                    : restartButton.hovered ? Theme.hover : "transparent"
                            }
                        }
                        Button {
                            id: shutDownButton
                            Layout.fillWidth: true
                            text: "Shut down"
                            Accessible.name: text
                            Keys.onEscapePressed: lockBackground.powerMenuOpen = false
                            onClicked: powerOffSystem.running = true
                            contentItem: Text {
                                text: shutDownButton.text
                                color: Theme.danger
                                font.family: Theme.sans
                                font.pixelSize: Theme.label
                                horizontalAlignment: Text.AlignLeft
                                verticalAlignment: Text.AlignVCenter
                            }
                            background: Rectangle {
                                radius: Theme.radiusChip
                                color: shutDownButton.down ? Theme.dangerTint
                                    : shutDownButton.hovered ? Theme.hover : "transparent"
                            }
                        }
                    }
                }
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
                        contentItem: Text {
                            leftPadding: showPassword.indicator.width + showPassword.spacing
                            text: showPassword.text
                            color: Theme.text
                            font: showPassword.font
                            verticalAlignment: Text.AlignVCenter
                        }
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
