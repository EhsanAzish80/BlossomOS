import QtQuick
import QtQuick.Controls
import "Petal"

Button {
    id: control
    required property var toplevel
    signal menuRequested()

    implicitWidth: 52
    implicitHeight: 52
    padding: 0
    Accessible.name: toplevel.title.length > 0 ? toplevel.title : toplevel.appId
    Accessible.description: toplevel.activated
        ? "Active window. Click to minimize; right-click for window controls."
        : "Open window. Click to focus; right-click for window controls."
    ToolTip.visible: hovered
    ToolTip.text: Accessible.name
    ToolTip.delay: 350

    onClicked: {
        if (toplevel.activated && !toplevel.minimized)
            toplevel.minimized = true
        else {
            toplevel.minimized = false
            toplevel.activate()
        }
    }

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Menu
                || (event.key === Qt.Key_F10
                    && (event.modifiers & Qt.ShiftModifier))) {
            control.menuRequested()
            event.accepted = true
        }
    }

    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: control.menuRequested()
    }

    contentItem: Item {
        Text {
            anchors.centerIn: parent
            anchors.verticalCenterOffset: -2
            text: {
                const label = control.toplevel.appId.length > 0
                    ? control.toplevel.appId : control.toplevel.title
                return label.length > 0 ? label.charAt(0).toUpperCase() : "□"
            }
            color: Theme.text
            font.family: Theme.sans
            font.pixelSize: 20
            font.bold: true
        }
        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            width: control.toplevel.activated ? 16 : 4
            height: 4
            radius: 2
            color: Theme.blossom
        }
    }

    background: Rectangle {
        radius: Theme.radiusTile
        color: control.down ? Theme.pressed
            : control.toplevel.activated ? Theme.blossomTint
            : control.hovered ? Theme.hover : "transparent"
        border.color: Theme.blossom
        border.width: control.activeFocus ? Theme.focusRing : 0
        Behavior on color { ColorAnimation { duration: Theme.stateMs } }
    }
}
