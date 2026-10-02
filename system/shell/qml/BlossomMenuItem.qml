import QtQuick
import QtQuick.Controls
import "Petal"

Button {
    id: control
    property bool destructive: false

    Accessible.role: Accessible.MenuItem
    Accessible.name: text

    implicitWidth: 210
    implicitHeight: 38
    leftPadding: 12
    rightPadding: 12
    topPadding: 0
    bottomPadding: 0

    contentItem: Text {
        text: control.text
        color: control.destructive && control.hovered ? Theme.danger : Theme.text
        font.family: Theme.sans
        font.pixelSize: Theme.body
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignLeft
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: Theme.radiusControl
        color: control.down ? Theme.pressed : control.hovered ? Theme.hover : "transparent"
        border.color: Theme.blossom
        border.width: control.activeFocus ? Theme.focusRing : 0
    }
}
