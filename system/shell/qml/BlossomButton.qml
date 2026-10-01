import QtQuick
import QtQuick.Controls
import "Petal"

Button {
    id: control
    property bool primary: false
    property bool destructive: false
    property bool ghost: false

    implicitHeight: Theme.controlHeight
    leftPadding: 18
    rightPadding: 18
    topPadding: 0
    bottomPadding: 0
    font.family: Theme.sans
    font.pixelSize: Theme.label
    font.weight: control.primary ? Font.DemiBold : Font.Medium

    contentItem: Text {
        text: control.text
        font: control.font
        color: !control.enabled ? Theme.textDisabled
             : control.primary ? Theme.onBlossom
             : control.destructive ? ((control.hovered || control.down) ? Theme.onDanger : Theme.danger)
             : control.ghost && !control.hovered ? Theme.textSecondary
             : Theme.text
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Item {
        Rectangle {
            anchors.fill: parent
            anchors.margins: -(Theme.focusRing * 2)
            radius: Theme.radiusControl + Theme.focusRing * 2
            color: "transparent"
            border.color: control.destructive ? Theme.danger : Theme.blossom
            border.width: Theme.focusRing
            visible: control.activeFocus
        }
        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusControl
            color: !control.enabled ? (control.ghost ? "transparent" : Theme.surface)
                 : control.primary ? (control.down ? Theme.blossomPressed : control.hovered ? Theme.blossomHover : Theme.blossom)
                 : control.destructive ? (control.down ? Theme.dangerPressed : control.hovered ? Theme.danger : Theme.dangerTint)
                 : control.down ? Theme.pressed
                 : control.hovered ? Theme.hover
                 : control.ghost ? "transparent" : Theme.raised
            border.width: control.primary || control.ghost || (control.destructive && control.hovered) ? 0 : 1
            border.color: control.destructive ? Theme.dangerLine : control.enabled ? Theme.lineStrong : Theme.line
            Behavior on color { ColorAnimation { duration: Theme.stateMs } }
        }
    }
}
