import QtQuick
import QtQuick.Controls
import QtQuick.Window
import Blossom.Shell

ApplicationWindow {
    id: securityHost
    visible: false
    width: 1
    height: 1

    function restoreRequestFocus() { securityHost.requestActivate() }

    Connections {
        target: BlossomBroker
        function onStateChanged() {
            if (!["waiting", "submitting", "cancelling"].includes(BlossomBroker.state))
                Qt.callLater(securityHost.restoreRequestFocus)
        }
    }

    Label {
        visible: false
        text: "Status: " + BlossomBroker.state
        Accessible.role: Accessible.AlertMessage
        Accessible.name: text
    }

    ApprovalPanel {}
}
